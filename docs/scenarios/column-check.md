# Column CHECK attribute

**Feature:** `#[column(check = "...")]` stores a raw column CHECK on `ColumnDef.check` as `CHECK (<expr>)`, the same shape the `ColumnEnum` path already produces, so DDL, snapshot extraction, and diff need no special case. Combining `check` with `as_text` is a compile error (one source of truth per column). Table-level `#[check]` is out of scope.
**Drivers:** none for the attribute itself (schema + SQL generation). Conflict pinned by trybuild under [Macro compile errors](macro-compile-errors.md).
**Issue:** #66.

## What is proven

- The attribute wraps the body: `check = "quality BETWEEN 1 AND 5"` → `Some("CHECK (quality BETWEEN 1 AND 5)")`.
- `CREATE TABLE` SQL from `column_def_sql` includes the CHECK clause unchanged.
- `Snapshot::from_registry` still lifts the expression into `check_constraints` and strips it from the column map (existing path).

## How to run

```sh
cargo nextest run -p toolu-orm-macros -E 'binary(column_check_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | column_check_test | column_check_attr_sets_wrapped_check |
| default | column_check_test | column_check_renders_in_create_table_sql |
| default | column_check_test | column_check_flows_through_snapshot_unchanged |
