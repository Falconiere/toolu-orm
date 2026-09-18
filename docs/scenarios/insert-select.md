# INSERT … SELECT and database-qualified table names

**Feature:** `InsertBuilder::select(&[&COL, …], source)` fills an `INSERT` from a whole `SELECT` instead of a `VALUES` list, so a set-based copy is one statement and no row is ever decoded into Rust. `TableRef::in_database(name)` qualifies a table reference structurally, so `main.indexed_files` renders as `"main"."indexed_files"` — two identifiers — rather than as `"main.indexed_files"`, one name containing a dot. `InsertBuilder::into_table(…)` takes such a reference as the target; `SelectBuilder::from_table(…)` already did.

**Drivers:** libsql, rusqlite, Postgres.
**Spec:** issue #114 (INSERT … SELECT with explicit target columns and a conflict mode, structured database/schema-qualified table references).

## The operation

comemory's runtime rebuild copies non-reconstructable rows out of an attached older store:

```sql
INSERT OR IGNORE INTO main.indexed_files(repo, path, blob_oid, indexed_at)
SELECT repo, path, blob_oid, indexed_at FROM old.indexed_files;
```

which is now:

```rust
let older = SelectBuilder::from_table(TableRef::new("indexed_files").in_database("old"))
  .columns_raw(&["repo", "path", "blob_oid", "indexed_at"])
  .column_scalar(Scalar::bind(7_i64), "rank"); // a column the older store lacks

InsertBuilder::into_table(TableRef::new("indexed_files").in_database("main"))
  .or_ignore()
  .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK], older)
  .execute(&conn)?;
```

`select` takes the **target** columns and the source together, so a column list that disagrees with the values is unrepresentable; it replaces anything `set()` recorded, whichever order the two were called in. `select_raw(&["repo", …], source)` is the same with the target columns spelled as strings — `_raw` qualifies the columns, not the statement. An empty target list renders `INSERT INTO "t" <select>`, matching positionally rather than emitting an invalid `()`. `returning(&COL)` projects one row per *inserted* row, so read it with `fetch_all`.

The source is any `SelectSource`, which `SelectBuilder` implements — including one carrying a `WITH` prefix, `UNION` arms, a `JOIN`, or `GROUP BY`. It is appended **unparenthesised**, because SQLite rejects `INSERT INTO "t" ("a") (SELECT …)` outright even though Postgres accepts it.

## What a database qualifier means per dialect

The two engines spell it identically and resolve it differently. Nothing in this library translates between them.

| | SQLite (libsql, rusqlite) | Postgres |
|---|---|---|
| `TableRef::new("t").in_database("old")` | `"old"."t"` | `"old"."t"` |
| What the first part names | an **attachment** — `main`, `temp`, or a name bound by `ATTACH DATABASE … AS` on *this connection*, addressing another file | a **namespace** — a catalog schema inside one database, which `search_path` would otherwise resolve |
| Chosen | at runtime, per connection | at schema-creation time |

`qualifier()` — what columns are addressed through — stays the alias, else the table, and is **never** the database: both engines resolve the two-part `"indexed_files"."repo"` against a `FROM "old"."indexed_files"` item, so `AliasedColumn` needs no third part. Every part doubles an embedded `"`, the only escape a delimited identifier has on either engine.

An `INSERT INTO` target renders its database and table and drops any alias, because `OnConflict`'s `Scalar::col(&COL)` renders `"table"."column"` from the column's own table and an alias would unname it.

## Per-dialect SQL

| Builder | SQLite | Postgres |
|---|---|---|
| `into_table(main_t).or_ignore().select(&[&A], src)` | `INSERT OR IGNORE INTO "main"."t" ("a") <src>` | `INSERT INTO "main"."t" ("a") <src> ON CONFLICT DO NOTHING` |
| `.select(&[&A], src)` | `INSERT INTO "main"."t" ("a") <src>` | same, `$N` |
| `.select(&[], src)` | `INSERT INTO "main"."t" <src>` | same |
| `.select(…).on_conflict(c)` | `… SELECT * FROM (<src>) AS "toolu_insert_source" WHERE true ON CONFLICT …` | `… <src> ON CONFLICT …` — **unwrapped** |
| `.select(…).returning(&ID)` | `… <src> RETURNING "id"` | same |

### Why SQLite wraps the source under an explicit `ON CONFLICT`

SQLite's parser cannot tell an upsert's `ON CONFLICT` from a join's `ON` when the insert source is a `SELECT` with a `FROM`. Measured on SQLite 3.51.0:

```text
sqlite> INSERT INTO "u" ("a","c") SELECT "a","c" FROM "s" ON CONFLICT ("a") DO UPDATE SET "c" = 42;
Parse error: near "DO": syntax error
```

SQLite's own documented workaround is to give the `SELECT` a `WHERE` clause. The builder cannot add one to an opaque `SelectSource` that may already carry `GROUP BY`, `ORDER BY`, `LIMIT` or a `UNION`, so it supplies the clause on a derived table instead. The wrapper binds nothing, so numbering is identical either way.

Postgres 16 parses every one of those shapes unwrapped — with `ON CONFLICT DO NOTHING`, with an explicit target, and over a source carrying its own `JOIN … ON` — so it is never wrapped; charging one engine's parser quirk to the other would be the library's divergence rather than the engines'. `or_ignore()` / `or_replace()` are never wrapped either: on SQLite they are `INSERT OR …` keywords, with no trailing `ON` to confuse.

## Parameter order

The statement keeps one ordered parameter list: every bind the **source** carries first, in the source's own order, then the `ON CONFLICT` clause's. `RETURNING` binds nothing. So a copy projecting a default and filtering, under an updating clause, renders on SQLite as

```sql
INSERT INTO "main"."indexed_files" ("repo", "path", "rank")
SELECT * FROM (
  SELECT "repo", "path", ?1 AS "rank" FROM "old"."indexed_files" WHERE "indexed_files"."repo" = ?2
) AS "toolu_insert_source" WHERE true
ON CONFLICT ("repo") DO UPDATE SET "indexed_at" = ?3
```

with `["legacy", "r1", 99]`, and on Postgres as the same statement unwrapped with `$1`/`$2`/`$3`.

## What is proven

### Against two real on-disk SQLite databases joined by ATTACH (rusqlite lane)

The source is a *different file*, reached only through `SqliteMaintenance::attach_database` (issue #115), whose guard detaches on every exit path. `old.db`'s `indexed_files` has **no** `rank` column; `main.db`'s does.

- **One statement, every row.** The builder's only call is `execute()`, which returns an affected-row count and no rows — three source rows, three affected. The rendered SQL is a single `INSERT … SELECT` with no `;`.
- **NULLs.** A row whose text and blob are NULL lands with both still NULL — not `""`, not `0`.
- **Blobs.** `[0x00, 0x10, 0xFF, 0x7F, 0x00, 0xC3]` — bytes no text encoding round-trips — comes back byte-identical, and an *empty* blob stays a value distinct from NULL.
- **Old-schema projected default.** `rank` is filled by the SELECT list (`?1 AS "rank"`), so every copied row has `7`, not the table's `DEFAULT -1`.
- **`INSERT OR IGNORE`.** Over a pre-seeded conflicting key it reports 2 affected of 3 and the stored row keeps its own `indexed_at` and `rank`. The same builder *without* `or_ignore()` returns `QueryError::Driver` naming the UNIQUE violation.
- **Explicit `ON CONFLICT`.** Parses and runs, updating the conflicting row from `excluded` — the statement that is a parse error without the guard.
- **A qualified catalogue read.** One `SelectBuilder` over `sqlite_master`, run twice with `in_database("old")` and `in_database("main")`, returns the two stores' different table sets. That is the presence check the rebuild runs before copying a table an older store may not have.
- **Empty source.** Reports `0` affected and leaves the target empty.

### Against in-memory libsql

The same shapes inside one database, where `main` is the qualifier: the copy with its NULLs, blobs and projected default; `or_ignore` keeping the stored row; the explicit clause updating it; and a qualified target landing exactly what the unqualified one does.

### Against live Postgres

The qualifier is a schema each test owns. `bytea` round-trips byte-identical including the empty one; `or_ignore` renders `ON CONFLICT DO NOTHING` and keeps the stored row; the explicit clause runs **unwrapped** (asserted, so a guard leaking into Postgres fails here); and a source binding `$1` under a clause binding `$2` lands `22` on the conflicting row and `11` on an inserted one — transposing them would be exactly the silent corruption that pins.

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | table_qualifier_test | a_qualified_relation_renders_two_identifiers_not_one_dotted_name |
| default | table_qualifier_test | the_qualifier_is_readable_and_the_last_call_wins |
| default | table_qualifier_test | columns_stay_qualified_by_the_alias_or_table_never_by_the_database |
| default | table_qualifier_test | every_part_doubles_an_embedded_quote |
| default | table_qualifier_test | a_qualified_table_function_keeps_its_arguments_numbered_from_start |
| default | table_qualifier_test | an_unqualified_table_ref_renders_exactly_what_it_rendered_before |
| default | insert_select_sql_test | rendering::the_issues_copy_renders_a_two_part_target_and_a_bare_select |
| default | insert_select_sql_test | rendering::postgres_renders_the_conflict_mode_as_a_clause_and_keeps_the_qualifier |
| default | insert_select_sql_test | rendering::a_projected_default_fills_a_column_the_source_does_not_have |
| default | insert_select_sql_test | rendering::an_empty_target_column_list_renders_no_empty_parentheses |
| default | insert_select_sql_test | rendering::select_replaces_any_recorded_values_whichever_order_they_were_called_in |
| default | insert_select_sql_test | rendering::select_raw_names_the_target_columns_as_strings |
| default | insert_select_sql_test | rendering::a_with_prefix_and_a_union_arm_are_appended_whole |
| default | insert_select_sql_test | rendering::an_identifier_carrying_a_quote_doubles_it_in_every_part |
| default | insert_select_sql_test | conflict::sqlite_wraps_the_source_so_on_conflict_cannot_be_read_as_a_join |
| default | insert_select_sql_test | conflict::postgres_needs_no_guard_and_does_not_get_one |
| default | insert_select_sql_test | conflict::the_keyword_conflict_modes_are_never_guarded_on_either_dialect |
| default | insert_select_sql_test | conflict::a_values_insert_with_an_explicit_clause_is_unchanged_by_the_guard |
| default | insert_select_sql_test | conflict::the_legacy_postgres_replace_targets_the_select_column_list |
| default | insert_select_sql_test | conflict::returning_renders_after_the_source_and_binds_nothing |
| default | insert_select_sql_test | bind_order::source_binds_precede_conflict_binds_on_sqlite |
| default | insert_select_sql_test | bind_order::source_binds_precede_conflict_binds_on_postgres |
| default | insert_select_sql_test | bind_order::the_indices_run_one_through_len_with_no_repeat_and_no_gap |
| default | insert_select_sql_test | bind_order::a_zero_bind_source_leaves_the_conflict_clause_starting_at_one |
| default | insert_select_sql_test | bind_order::the_guard_wrapper_itself_consumes_no_index |
| rusqlite-only | rusqlite_insert_select_test | copy::one_statement_copies_every_row_across_the_attachment |
| rusqlite-only | rusqlite_insert_select_test | copy::a_null_text_and_a_null_blob_survive_the_copy_as_nulls |
| rusqlite-only | rusqlite_insert_select_test | copy::a_blob_round_trips_byte_identical_including_an_empty_one |
| rusqlite-only | rusqlite_insert_select_test | copy::a_column_the_source_lacks_takes_the_projected_default |
| rusqlite-only | rusqlite_insert_select_test | copy::the_executed_statement_is_a_single_insert_select |
| rusqlite-only | rusqlite_insert_select_test | copy::a_source_with_no_rows_reports_zero_and_leaves_the_target_untouched |
| rusqlite-only | rusqlite_insert_select_test | conflict::or_ignore_skips_the_conflicting_row_and_keeps_the_stored_one |
| rusqlite-only | rusqlite_insert_select_test | conflict::without_a_conflict_mode_the_same_copy_reports_the_uniqueness_violation |
| rusqlite-only | rusqlite_insert_select_test | conflict::an_explicit_conflict_clause_parses_and_updates_the_stored_row |
| rusqlite-only | rusqlite_insert_select_test | catalogue::the_qualifier_selects_which_databases_catalogue_is_read |
| libsql-only | libsql_insert_select_test | copy::one_statement_copies_every_row_with_its_nulls_and_blobs |
| libsql-only | libsql_insert_select_test | copy::or_ignore_keeps_the_stored_row_and_copies_the_rest |
| libsql-only | libsql_insert_select_test | copy::an_explicit_conflict_clause_parses_and_updates_the_stored_row |
| libsql-only | libsql_insert_select_test | copy::the_unqualified_target_lands_the_same_rows_as_the_qualified_one |
| postgres | postgres_insert_select_test | copy::one_statement_copies_every_row_across_the_schema_qualifier |
| postgres | postgres_insert_select_test | copy::or_ignore_becomes_on_conflict_do_nothing_and_keeps_the_stored_row |
| postgres | postgres_insert_select_test | copy::the_explicit_conflict_clause_runs_unwrapped_on_postgres |
| postgres | postgres_insert_select_test | copy::a_binding_source_and_a_binding_conflict_clause_keep_their_order |
