# Legacy snapshot

**Feature:** `*.snapshot.json` files written by older versions still load. `serde_compat` accepts a table whose `columns` is a JSON array (order inferred from the array) and whose `column_order`, `indexes`, `foreign_keys`, `check_constraints`, and `strict` keys are absent.
**Drivers:** dialect-independent (snapshot and diff engine).
**Spec:** AC-14.

## What is proven

Fixture: `crates/orm-core/tests/fixtures/legacy_snapshot.json`, a real file in the old shape with `users(id, name)` and `posts(id, author_id)`.

- `Snapshot::read_from_path` succeeds; each table's `column_order` equals the array order; `indexes`, `foreign_keys`, `check_constraints` are empty; `strict` is false.
- Columns are keyed by name after loading, so later lookups by name work.
- `diff(&legacy, &registry)` where the registry is identical except that `posts.author_id` gains `references = "users(id)"` yields exactly one operation, the add-foreign-key variant. Nothing else is misread as a change.
- The fixture also predates virtual tables: it carries no `kind` on a table and no `unindexed` on a column. Both default, so every table loads as `TableKind::Ordinary` with indexed columns — see [Virtual tables](virtual-tables.md).

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(snapshot_legacy_shapes_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | snapshot_legacy_shapes_test | legacy_array_columns_infer_column_order_and_defaults |
| default | snapshot_legacy_shapes_test | legacy_snapshot_columns_are_keyed_by_name |
| default | snapshot_legacy_shapes_test | legacy_tables_default_to_ordinary_kind_and_indexed_columns |
| default | snapshot_legacy_shapes_test | diff_against_legacy_snapshot_yields_single_add_foreign_key |
