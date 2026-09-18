# Upsert and plain mutations

**Feature:** `InsertBuilder` / `UpdateBuilder` / `DeleteBuilder` share `.execute(&exec)` and return the affected-row count. Two conflict surfaces exist and a statement carries exactly one of them — the later call wins:

- **Explicit (issue #108):** `on_conflict(OnConflict::column(&ID)…)` builds `ON CONFLICT (<cols>) DO NOTHING | DO UPDATE SET …`, rendered identically on SQLite and Postgres. `returning(&COL)` adds a `RETURNING` projection, read back with `fetch_one` / `fetch_optional` / `fetch_all`.
- **Legacy shorthand:** `or_replace()` and `or_ignore()`, with `conflict_columns(&[...])` naming the Postgres target of `or_replace` (defaults to the first inserted column; SQLite's `INSERT OR REPLACE` takes no target).

**Drivers:** libsql, rusqlite, Postgres.
**Spec:** AC-4 (upsert), AC-3 (Postgres CRUD), issue #108 (explicit conflict clause, expression values, `RETURNING`).

## SQL per dialect

| Builder | SQLite (libsql, rusqlite) | Postgres |
|---|---|---|
| `insert().or_replace().conflict_columns(&["id"])` | `INSERT OR REPLACE INTO "users" (...) VALUES (?1, ...)` | `INSERT INTO "users" (...) VALUES ($1, ...) ON CONFLICT ("id") DO UPDATE SET "name" = EXCLUDED."name", ...` |
| `insert().or_ignore()` | `INSERT OR IGNORE INTO ...` | `INSERT INTO ... ON CONFLICT DO NOTHING` |
| `update().set(&NAME, "x").set_expr(&AGE, "\"age\" + 1").filter(ID.eq("u1"))` | `UPDATE "users" SET "name" = ?1, "age" = "age" + 1 WHERE "users"."id" = ?2` | same with `$N` |
| `delete().filter(ID.eq("u1"))` | `DELETE FROM "users" WHERE "users"."id" = ?1` | same with `$1` |

### The explicit conflict clause

| Built from | SQLite and Postgres (one rendering, `?N` / `$N` apart) |
|---|---|
| `on_conflict(OnConflict::column(&ID))` | `… ON CONFLICT ("id") DO NOTHING` |
| `.and_column(&PATH)` | `… ON CONFLICT ("repo", "path") DO …` |
| `.set(&LAST_USED, "t1")` | `DO UPDATE SET "last_used" = ?N` |
| `.set_scalar(&USED, Scalar::col(&USED) + Scalar::bind(1))` | `DO UPDATE SET "used_count" = ("memories"."used_count" + ?N)` |
| `.set_excluded(&BODY)` | `DO UPDATE SET "body" = "excluded"."body"` |
| `Scalar::func("coalesce", [Scalar::col(&WS), Scalar::excluded(&WS)])?` | `coalesce("memories"."workspace_id", "excluded"."workspace_id")` |
| `.do_nothing()` after assignments | `DO NOTHING` — the recorded assignments are discarded |
| `.returning(&ID).returning(&PATH)` | `… RETURNING "id", "path"` |
| any identifier carrying a `"` | the interior quote **doubles** (`"wo""rkspace"`), the only escape a delimited identifier has on either engine |

Inside the clause `Scalar::col` names the **stored** row and `Scalar::excluded` the one the `INSERT` proposed. The whole statement keeps one ordered parameter list: every `VALUES` bind first, then every `DO UPDATE` bind, in call order; `RETURNING` binds nothing. So the `code_row.rs` statement from issue #108 renders as

```sql
INSERT INTO "code_symbols" ("repo", "path", "indexed_at")
VALUES (?1, ?2, strftime(?3, ?4))
ON CONFLICT ("repo", "path") DO UPDATE SET "indexed_at" = strftime(?5, ?6)
RETURNING "id"
```

Postgres gets that same clause rather than an approximation: only the placeholder style changes. The legacy `or_replace()` keeps its per-dialect divergence, documented above and unchanged.

## What is proven

- Inserting `("u1", "Ann")` then `("u1", "Bea")` with `or_replace` leaves exactly one row named `Bea` on all three drivers; Postgres reports 1 affected row for the `DO UPDATE`.
- The same with `or_ignore` keeps `Ann`; Postgres reports 0 affected rows.
- Postgres: insert returns 1, `update` with `set` + `set_expr` changes the row (`age` 30 to 31), `delete` returns 1 and the table is empty afterwards. The SQLite CRUD path is covered by the revived `executor_test` and `integration_test` (see [Lanes](lanes.md)).

### Explicit conflict clause, against real databases

Fixture (`tests/fixtures/upsert_schema.rs`): `memories(id, body, workspace_id, used_count, last_used)`, `memory_links(id, memory_id REFERENCES memories(id) ON DELETE CASCADE, note)`, and `code_symbols(id generated, repo, path, indexed_at, UNIQUE(repo, path))`. The SQLite suites set `PRAGMA foreign_keys = ON` and assert it reads back `1`, because the cascade below is the whole point of the contrast.

Starting from `m1 = ('original', 'w1', used_count 3, 't0')`:

- **Partial preservation and counters.** An upsert that names only `used_count` and `last_used` leaves `used_count = 4`, `last_used = 't1'`, and `body`/`workspace_id` exactly as stored. The same builder on a *fresh* row writes `used_count = 1`, so the increment is provably the conflict branch.
- **`DO NOTHING`.** Reports `0` affected rows and the whole decoded row is unchanged; on a non-conflicting key the same builder still inserts and reports `1`.
- **Foreign-key safety.** With a real `memory_links` child row, the upsert leaves it in place. `or_replace()` on the same fixture deletes it — `ON DELETE CASCADE` fires — and resets `used_count` to its default and `workspace_id` to NULL. That is the delete/re-insert difference issue #108 is about.
- **Generated ids.** `.returning(&SYMBOL_ID).fetch_one::<GeneratedId>()` gives `1`, then `2` for a distinct row, then `1` again for the conflicting re-insert, which also takes the proposed `indexed_at`. `or_replace()` on the same row hands back `2` instead — a brand new id.
- **`excluded` and `COALESCE`.** `set_excluded(&BODY)` takes the incoming body while `coalesce(stored, excluded)` keeps `w1` when the incoming `workspace_id` is NULL; reversing the arguments adopts the incoming `w2`.
- **Expression values.** `strftime('%Y-%m-%dT%H:%M:%fZ','now')` with both arguments bound writes a 24-character ISO-8601 timestamp ending in `Z`, and the `DO UPDATE` branch evaluates its own copy — a row seeded at the epoch comes back strictly later.
- **Fetch semantics.** `fetch_optional` is `None` when `DO NOTHING` suppressed the write; `fetch_one` on a statement with no `RETURNING` is `QueryError::NotFound` while the row is still inserted. On rusqlite, `execute()` on a `RETURNING` statement is a driver error (`Execute returned results`) — use a fetch method.
- **Postgres.** The identical builder chain gives the same outcomes on the live server, with `BIGINT GENERATED BY DEFAULT AS IDENTITY` for the generated id.

## How to run

```sh
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_mutations_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_mutations_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(postgres_mutations_test)'

# The explicit conflict clause
cargo nextest run -p toolu-orm-query -E 'binary(upsert_sql_test)'
cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec -E 'binary(rusqlite_upsert_test)'
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_upsert_test)'
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-query --features postgres -E 'binary(postgres_upsert_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| libsql-only | libsql_mutations_test | or_replace_replaces_conflicting_row |
| libsql-only | libsql_mutations_test | or_ignore_keeps_original_row |
| rusqlite-only | rusqlite_mutations_test | or_replace_replaces_conflicting_row |
| rusqlite-only | rusqlite_mutations_test | or_ignore_keeps_original_row |
| postgres | postgres_mutations_test | insert_update_delete_round_trip |
| postgres | postgres_mutations_test | or_replace_updates_the_conflicting_row |
| postgres | postgres_mutations_test | or_ignore_keeps_the_existing_row |
| default | upsert_sql_test | binding::a_clause_numbers_from_one_when_the_values_bind_nothing |
| default | upsert_sql_test | binding::a_raw_assignment_renumbers_its_own_placeholders_after_the_values |
| default | upsert_sql_test | binding::expression_values_then_expression_assignments_number_left_to_right |
| default | upsert_sql_test | binding::the_same_statement_numbers_dollar_placeholders_in_the_same_order |
| default | upsert_sql_test | clause::a_clause_without_assignments_renders_do_nothing |
| default | upsert_sql_test | clause::an_embedded_double_quote_in_an_identifier_doubles_rather_than_escaping |
| default | upsert_sql_test | clause::a_composite_target_lists_its_columns_in_call_order |
| default | upsert_sql_test | clause::a_counter_update_reads_the_existing_row_and_binds_after_the_values |
| default | upsert_sql_test | clause::a_later_or_replace_replaces_the_clause_and_a_later_clause_replaces_it |
| default | upsert_sql_test | clause::do_nothing_discards_every_assignment_recorded_before_it |
| default | upsert_sql_test | clause::excluded_and_coalesce_assignments_render_the_upsert_vocabulary |
| default | upsert_sql_test | clause::the_same_upsert_renders_with_dollar_placeholders_on_postgres |
| default | upsert_sql_test | returning::no_returning_call_leaves_the_statement_unchanged |
| default | upsert_sql_test | returning::returning_also_follows_the_legacy_or_ignore_shorthand |
| default | upsert_sql_test | returning::returning_follows_the_conflict_clause_on_postgres |
| default | upsert_sql_test | returning::returning_projects_its_columns_unqualified_in_call_order |
| rusqlite-only | rusqlite_upsert_test | clock::a_database_clock_expression_writes_an_iso_timestamp |
| rusqlite-only | rusqlite_upsert_test | clock::the_conflict_branch_recomputes_the_clock_from_its_own_binds |
| rusqlite-only | rusqlite_upsert_test | counters::a_conflict_increments_the_counter_and_leaves_unnamed_columns_alone |
| rusqlite-only | rusqlite_upsert_test | counters::coalesce_keeps_a_stored_workspace_and_adopts_a_missing_one |
| rusqlite-only | rusqlite_upsert_test | counters::do_nothing_reports_no_affected_row_and_changes_nothing |
| rusqlite-only | rusqlite_upsert_test | counters::do_nothing_still_inserts_a_row_that_does_not_conflict |
| rusqlite-only | rusqlite_upsert_test | counters::the_first_insert_writes_the_values_rather_than_the_update |
| rusqlite-only | rusqlite_upsert_test | generated_ids::execute_refuses_a_returning_statement_on_rusqlite |
| rusqlite-only | rusqlite_upsert_test | generated_ids::fetch_all_returns_the_one_projected_row_and_an_empty_vector_when_suppressed |
| rusqlite-only | rusqlite_upsert_test | generated_ids::fetch_one_without_a_returning_clause_reports_not_found |
| rusqlite-only | rusqlite_upsert_test | generated_ids::fetch_optional_is_none_when_do_nothing_suppressed_the_write |
| rusqlite-only | rusqlite_upsert_test | generated_ids::or_replace_allocates_a_new_id_where_do_update_kept_it |
| rusqlite-only | rusqlite_upsert_test | generated_ids::returning_hands_back_the_generated_id_and_keeps_it_across_a_conflict |
| rusqlite-only | rusqlite_upsert_test | references::an_upsert_keeps_the_rows_that_reference_the_conflicting_row |
| rusqlite-only | rusqlite_upsert_test | references::an_upsert_that_raises_the_counter_still_keeps_the_child_row |
| rusqlite-only | rusqlite_upsert_test | references::or_replace_cascades_the_referencing_row_away |
| rusqlite-only | rusqlite_upsert_test | references::the_fixture_really_enforces_foreign_keys |
| libsql-only | libsql_upsert_test | clock::a_database_clock_expression_writes_an_iso_timestamp |
| libsql-only | libsql_upsert_test | clock::the_conflict_branch_recomputes_the_clock_from_its_own_binds |
| libsql-only | libsql_upsert_test | counters::a_conflict_increments_the_counter_and_leaves_unnamed_columns_alone |
| libsql-only | libsql_upsert_test | counters::coalesce_keeps_a_stored_workspace_and_adopts_a_missing_one |
| libsql-only | libsql_upsert_test | counters::do_nothing_reports_no_affected_row_and_changes_nothing |
| libsql-only | libsql_upsert_test | counters::the_first_insert_writes_the_values_rather_than_the_update |
| libsql-only | libsql_upsert_test | generated_ids::fetch_one_without_a_returning_clause_reports_not_found |
| libsql-only | libsql_upsert_test | generated_ids::fetch_optional_is_none_when_do_nothing_suppressed_the_write |
| libsql-only | libsql_upsert_test | generated_ids::or_replace_allocates_a_new_id_where_do_update_kept_it |
| libsql-only | libsql_upsert_test | generated_ids::returning_hands_back_the_generated_id_and_keeps_it_across_a_conflict |
| libsql-only | libsql_upsert_test | references::an_upsert_keeps_the_rows_that_reference_the_conflicting_row |
| libsql-only | libsql_upsert_test | references::or_replace_cascades_the_referencing_row_away |
| libsql-only | libsql_upsert_test | references::the_fixture_really_enforces_foreign_keys |
| postgres | postgres_upsert_test | a_conflict_increments_the_counter_and_leaves_unnamed_columns_alone |
| postgres | postgres_upsert_test | an_upsert_keeps_the_rows_that_reference_the_conflicting_row |
| postgres | postgres_upsert_test | do_nothing_reports_no_affected_row_and_changes_nothing |
| postgres | postgres_upsert_test | excluded_and_coalesce_preserve_the_stored_workspace |
| postgres | postgres_upsert_test | fetch_optional_is_none_when_do_nothing_suppressed_the_write |
| postgres | postgres_upsert_test | returning_hands_back_the_identity_id_and_keeps_it_across_a_conflict |
