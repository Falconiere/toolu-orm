# Relational loads

**Feature:** `RelationalQuery::new(table, columns).with_many(...)` / `.with_one(...)` builds one statement that returns the parent's scalar columns plus one JSON column per relation; `#[derive(Relational)]` structs rebuild themselves from those values with `from_relational_values`. No N+1.
**Drivers:** libsql, rusqlite, Postgres.
**Spec:** AC-5. (`#[many_to_many]` is accepted by the derive but has no query-side API yet, spec Q5.)

## SQL per dialect

| | SQLite (libsql, rusqlite) | Postgres |
|---|---|---|
| `with_many("posts", "posts", "id", "author_id", ["id", "title"])` | correlated subquery with `json_group_array(json_array(...))` | `LEFT JOIN LATERAL (SELECT coalesce(json_agg(json_build_array(...)), '[]') ...)` |
| `with_one("author", "users", "author_id", "id", ["id", "name"])` | subquery with `json_array(...)`, `NULL` when unmatched | `LEFT JOIN LATERAL (SELECT json_build_array(...) ...)`, `NULL` when unmatched |
| JSON column type | text, parsed with `parse_many_column` / `parse_one_column` | `json`, read as `serde_json::Value` |

## Binary columns (`BLOB` / `bytea`)

Relation columns travel through JSON, and JSON has no binary type: SQLite
refuses a `BLOB` argument to `json_array` (*"JSON cannot hold BLOB values"*) and
Postgres renders `bytea` through its text output as `"\\x0102"`. A relation
column that holds binary data is therefore **declared**, and the declaration
drives both the SQL and the decoding:

```rust
use toolu_orm_query::select::RelationColumn;

let q = RelationalQuery::<(OwnerWithFiles,)>::new("owners", &["id"])
  .with_many_columns::<FileRow>(
    "files", "files", "id", "owner_id",
    &[RelationColumn::new("id"), RelationColumn::binary("payload")],
  );
let files = q.decode_relation_json("files", &files_json)?;  // SQLite: JSON text
let files = q.decode_relation_value("files", &json_column)?; // Postgres: `json`
```

`with_one_columns` is the has-one twin. The decoded column is handed straight to
`from_relational_values`, and the struct field is a plain `Vec<u8>` /
`Option<Vec<u8>>`.

| | SQLite (libsql, rusqlite) | Postgres |
|---|---|---|
| Declared binary column | `CASE WHEN "t"."c" IS NULL THEN NULL ELSE hex("t"."c") END` | `encode("t"."c", 'hex')` |
| Why the `CASE` | `hex(NULL)` is `''`, which would collapse NULL into the empty blob | `encode` is strict, so NULL stays NULL |
| Undeclared column | unchanged SQL; a `BLOB` fails at the database | unchanged SQL; `bytea` arrives as `"\\x…"` text |

Supported projected types: anything JSON holds natively — text, integer, real,
boolean, null — plus binary through `RelationColumn::binary`. Decoding never
guesses: text at an undeclared index is passed through untouched, so an ordinary
string that happens to look like hex is never mistaken for bytes. A column
declared binary that is not `BLOB` / `bytea` is the caller's error: SQLite hexes
the value's UTF-8 bytes, Postgres rejects `encode(text, 'hex')`.

Cost: a decoded binary value is a JSON array of byte numbers, so the relation
column is several times the blob's size on the wire. Large payloads are better
fetched with a separate query.

## What is proven

Seed: `u1 Ann` with posts `p1 Hello`, `p2 World`; `u2 Bea` with no posts; `p3 Orphan` with a `NULL` author.

- `with_many`: Ann's struct has both posts, Bea's has an empty `Vec`.
- `with_one`: `p1.author == Some(Ann)`, `p3.author == None`.
- The generated SQL is executed against the real database; the JSON is decoded through the same helpers a consumer would use.
- The SQL shape per dialect and the JSON parsing helpers are also pinned without a database (`relational_execute_test`).

Binary seed: owner `o1` with files `f1` (`X'0102FF007F'`), `f2` (`X''`) and `f3`
(`NULL`); owner `o2` with no files and a `NULL` avatar; file `f4` with no owner.

- `f1` decodes to the exact five bytes, `f2` to an empty `Vec<u8>`, `f3` to
  `None` — empty and NULL never collapse into each other.
- The has-one twin decodes the parent's binary column and still reports `None`
  for an unmatched relation.
- Dropping the declaration reproduces the original failure on purpose: SQLite
  and libsql raise *"JSON cannot hold BLOB values"*, and Postgres returns the
  `\x`-prefixed text, while the declared projection returns bytes.
- Decode failures (unknown relation, invalid hex, a non-string value at a binary
  index, a row whose arity differs from the projection) are `RowMapping` errors
  that name the relation and column.

## How to run

```sh
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_relational_test) + binary(libsql_relational_blob_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_relational_test) + binary(rusqlite_relational_blob_test)'
cargo nextest run --workspace -E 'binary(relational_binary_sql_test) + binary(relational_binary_decode_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(postgres_relational_test) + binary(postgres_relational_blob_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| libsql-only | libsql_relational_test | with_many_builds_structs_via_from_relational_values |
| libsql-only | libsql_relational_test | with_many_reports_correct_post_counts_via_parse_many_column |
| libsql-only | libsql_relational_test | with_one_returns_none_for_orphan_post_and_some_for_authored_post |
| rusqlite-only | rusqlite_relational_test | with_many_builds_structs_via_from_relational_values |
| rusqlite-only | rusqlite_relational_test | with_many_reports_correct_post_counts_via_parse_many_column |
| rusqlite-only | rusqlite_relational_test | with_one_returns_none_for_orphan_post_and_some_for_authored_post |
| postgres | postgres_relational_test | with_many_loads_children_as_one_statement |
| postgres | postgres_relational_test | with_one_loads_parent_and_null_parent |
| default | relational_execute_test | to_sql_postgres_generates_valid_sql |
| default | relational_execute_test | to_sql_sqlite_generates_valid_sql |
| default | relational_execute_test | deserialize_many_relation_from_json |
| default | relational_execute_test | deserialize_empty_many_relation_from_json |
| libsql-only | libsql_relational_blob_test | with_many_round_trips_nonempty_empty_and_null_blobs |
| libsql-only | libsql_relational_blob_test | with_one_round_trips_a_binary_column_and_reports_an_orphan |
| libsql-only | libsql_relational_blob_test | with_one_reports_a_null_binary_column_on_a_matched_parent |
| libsql-only | libsql_relational_blob_test | an_undeclared_blob_column_is_still_rejected_by_libsql |
| rusqlite-only | rusqlite_relational_blob_test | with_many_round_trips_nonempty_empty_and_null_blobs |
| rusqlite-only | rusqlite_relational_blob_test | with_one_round_trips_a_binary_column_and_reports_an_orphan |
| rusqlite-only | rusqlite_relational_blob_test | with_one_reports_a_null_binary_column_on_a_matched_parent |
| rusqlite-only | rusqlite_relational_blob_test | an_undeclared_blob_column_is_still_rejected_by_sqlite |
| postgres | postgres_relational_blob_test | with_many_round_trips_nonempty_empty_and_null_bytea |
| postgres | postgres_relational_blob_test | with_one_round_trips_bytea_and_reports_an_orphan |
| postgres | postgres_relational_blob_test | an_undeclared_bytea_column_arrives_as_escape_text |
| default | relational_binary_sql_test | sqlite_wraps_a_binary_column_in_a_null_preserving_hex_case |
| default | relational_binary_sql_test | sqlite_has_one_wraps_a_binary_column_too |
| default | relational_binary_sql_test | postgres_encodes_a_binary_column_as_hex |
| default | relational_binary_sql_test | postgres_has_one_encodes_a_binary_column_too |
| default | relational_binary_sql_test | a_projection_without_binary_columns_emits_no_encoder |
| default | relational_binary_sql_test | relation_columns_keep_their_declaration_in_the_config |
| default | relational_binary_decode_test | many_column_decodes_hex_to_bytes_and_leaves_other_values_alone |
| default | relational_binary_decode_test | lowercase_hex_from_postgres_decodes_to_the_same_bytes |
| default | relational_binary_decode_test | empty_many_column_decodes_to_an_empty_array |
| default | relational_binary_decode_test | one_column_decodes_hex_and_maps_a_missing_relation_to_null |
| default | relational_binary_decode_test | unknown_relation_field_is_reported |
| default | relational_binary_decode_test | non_hex_text_at_a_binary_index_is_reported |
| default | relational_binary_decode_test | a_non_text_value_at_a_binary_index_is_reported |
| default | relational_binary_decode_test | a_row_whose_arity_differs_from_the_projection_is_reported |
| default | relational_binary_decode_test | a_malformed_json_shape_is_reported |
| default | relational_binary_decode_test | a_projection_without_binary_columns_passes_every_value_through |
