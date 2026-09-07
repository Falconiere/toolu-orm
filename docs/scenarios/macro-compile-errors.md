# Macro compile errors

**Feature:** the proc macros reject malformed input with a specific message at the offending span, so a typo in `#[table]`, `#[index]`, `#[view]`, `#[derive(Relational)]`, or a relation attribute fails the build with an explanation instead of expanding into something else.
**Drivers:** none (compile time). Pinned with `trybuild`: each case in `crates/orm-macros/tests/ui/*.rs` must fail with exactly its checked-in `.stderr`.
**Spec:** AC-13.

## What is proven

| Case file | Input | Message |
|---|---|---|
| `table_missing_name.rs` | `#[table]` without `name` | missing `name` in #[table(name = "...")] |
| `table_unknown_attr.rs` | `#[table(name = "t", foo = "x")]` | unknown attribute, expected `name` or `strict` |
| `table_name_not_string.rs` | `#[table(name = 1)]` | expected a string literal |
| `table_strict_not_bool.rs` | `#[table(name = "t", strict = maybe)]` (a non-literal value; a string literal reaches a later check, "expected true or false") | expected a bool literal |
| `index_name_not_string.rs` | `#[index(123)]` | first arg must be index name string |
| `view_unknown_mode.rs` | `#[view(V, drop(a))]` | expected `omit` or `pick` |
| `relational_on_enum.rs` | `#[derive(Relational)]` on an enum | #[derive(Relational)] only works on structs |
| `relational_missing_table.rs` | struct without `#[relational(table = ...)]` | #[derive(Relational)] requires #[relational(table = "...")] |
| `has_many_on_string.rs` | `#[has_many]` on a `String` field | relation field must be Vec<T> or Option<T> |
| `many_to_many_missing_through.rs` | `#[many_to_many]` without `through` | missing `through` |
| `fts5_missing_name.rs` | `#[fts5_table]` without `name` | missing `name` in #[fts5_table(name = "...")] |
| `fts5_unknown_attr.rs` | `#[fts5_table(name = "t", strict = "yes")]` | unknown attribute, expected `name`, `tokenize`, `prefix`, `content`, `content_rowid`, `columnsize` or `detail` |
| `fts5_index_attr.rs` | `#[index(...)]` on an FTS5 struct | virtual tables cannot declare indexes; remove it from #[fts5_table] |
| `fts5_path_attr.rs` | `#[fts5_table(fts5::tokenize = "porter")]` | expected a simple identifier |
| `fts5_column_constraint.rs` | `#[column(unindexed, primary_key)]` on an FTS5 column | `primary_key` on `memory_id`: an fts5 column carries no constraints, only #[column(unindexed)] |

Not covered because the macros do not validate them today (spec Q6): unknown `#[column]` attributes, unknown `column_type` strings, invalid `on_delete` values, indexes on unknown columns, `pick` of unknown fields.

## How to run

```sh
cargo nextest run -p toolu-orm-macros -E 'binary(compile_fail_test)'
# after a toolchain bump, regenerate and review the diffs:
TRYBUILD=overwrite cargo nextest run -p toolu-orm-macros -E 'binary(compile_fail_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | compile_fail_test | compile_fail |
