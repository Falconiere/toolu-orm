# Index column DESC

**Feature:** `IndexDef.columns` is `Vec<IndexColumn { name, desc }>`. `#[index("…", desc(col))]` / `#[unique_index(...)]` accept `desc(column)`; `create_index_sql` renders `"col" DESC`; a direction change diffs as drop + create. Snapshots that still store `"columns": ["col"]` load with `desc: false`.
**Drivers:** SQLite and Postgres share the `DESC` syntax.
**Issue:** [#70](https://github.com/Falconiere/toolu-orm/issues/70).

## What is proven

- `desc(at)` on `#[index]` / `#[unique_index]` reaches `IndexColumn { name: "at", desc: true }` (and mixed ascending + descending lists).
- `CREATE INDEX … ("at" DESC)` on both dialects; ascending columns stay without a direction keyword.
- Legacy JSON `{"columns": ["at", "id"]}` deserializes; new wire form round-trips and omits `"desc": false`.
- Changing only the direction of an existing index yields `DropIndex` then `CreateIndex` with the DESC column.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(index_desc_test)'
cargo nextest run -p toolu-orm-macros -E 'binary(index_desc_macro_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | index_desc_test | create_index_sql_renders_desc_suffix |
| default | index_desc_test | create_index_sql_keeps_ascending_without_desc |
| default | index_desc_test | index_column_deserializes_legacy_string_list |
| default | index_desc_test | index_column_roundtrips_desc_object |
| default | index_desc_test | diff_direction_change_is_drop_then_create |
| default | index_desc_macro_test | desc_columns_reach_table_def |
