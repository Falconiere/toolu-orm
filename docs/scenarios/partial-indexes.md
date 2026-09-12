# Partial indexes

**Feature:** `IndexDef.where_clause` carries an optional `WHERE` predicate. Declare it with `#[index("name", col, where = "…")]` or `#[unique_index(..., where = "…")]`; `create_index_sql` appends ` WHERE <predicate>` on SQLite and Postgres; a changed predicate diffs as drop + create.
**Drivers:** both dialects (shared `CREATE INDEX … WHERE` syntax). Schema and SQL only — no live database.
**Issue:** [#67](https://github.com/Falconiere/toolu-orm/issues/67).

## What is proven

| Concern | Proof |
|---|---|
| Macro parse | `where = "deleted_at IS NULL"` lands verbatim on `IndexDef.where_clause` for both `#[index]` and `#[unique_index]`; an index without `where` keeps `None`. |
| SQL render | `CREATE INDEX` / `CREATE UNIQUE INDEX` append ` WHERE deleted_at IS NULL` on both dialects. |
| Legacy serde | JSON without `where_clause` deserializes to `None`; serializing `None` omits the key. |
| Diff | Changing only the predicate yields `DropIndex` then `CreateIndex` with the new predicate. |

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(partial_index_test)'
cargo nextest run -p toolu-orm-macros -E 'binary(partial_index_macro_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | partial_index_test | create_index_sql_appends_where_predicate |
| default | partial_index_test | create_unique_index_sql_appends_where_predicate |
| default | partial_index_test | legacy_index_json_without_where_clause_deserializes |
| default | partial_index_test | changed_where_predicate_diffs_as_drop_and_create |
| default | partial_index_macro_test | index_attr_parses_where_predicate |
