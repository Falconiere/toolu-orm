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

## What is proven

Seed: `u1 Ann` with posts `p1 Hello`, `p2 World`; `u2 Bea` with no posts; `p3 Orphan` with a `NULL` author.

- `with_many`: Ann's struct has both posts, Bea's has an empty `Vec`.
- `with_one`: `p1.author == Some(Ann)`, `p3.author == None`.
- The generated SQL is executed against the real database; the JSON is decoded through the same helpers a consumer would use.
- The SQL shape per dialect and the JSON parsing helpers are also pinned without a database (`relational_execute_test`).

## How to run

```sh
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(libsql_relational_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_relational_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(postgres_relational_test)'
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
