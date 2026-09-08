# vec0 virtual tables

**Feature:** `ColumnType::Vector { element, dim }`, the `Vec0Table` builder, and
`#[vec0_table]` model sqlite-vec `vec0` tables over the shared
`TableKind::Virtual` mechanism; `Value::vector` / `vector_with_dim` encode
little-endian f32 embeddings; the migration runner maps a driver's
`no such module: vec0` into `MigrateError::MissingExtension`.
**Drivers:** schema / DDL / diff / macro on every lane; the missing-extension
path is proven against real in-memory libsql (which has no `sqlite-vec`);
`Value::vector` round-trips through a real rusqlite BLOB column. Executing
`vec0` DDL itself needs the extension registered on the connection before
`run_migrate` (issue #12) — this workspace does not link it.
**Spec:** [vec0 virtual tables](../toolu/specs/2026-09-07-vec0-virtual-tables-design.md),
AC-1 … AC-11.

## What is proven

### The shape of a vec0 table

`Vec0Table` renders module arguments in declaration order — primary key,
vector (`float[N] distance_metric=…`), partition key, metadata, auxiliary
(`+column`) — without quoting identifiers inside the parentheses, because
`vec0`'s own scanner has no quoting. Hostile names are refused instead.
`ColumnType::Vector` survives the snapshot so a dimension change is visible to
the diff.

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS "memory_vec" USING "vec0"(
  memory_id text primary key,
  embedding float[1024] distance_metric=cosine,
  user_id integer partition key,
  label text,
  +contents text
);
```

### Diff refusals and the missing extension

SQLite has no `ALTER` for virtual tables, so changing `dim`, the element type,
or `distance_metric` is refused with `DbCoreError::VirtualTableChange` and no
migration file is written. Applying a generated `vec0` migration on a
connection that has not loaded `sqlite-vec` fails as
`MigrateError::MissingExtension { module: "vec0", … }` and rolls back; an
FTS5-only migration on the same driver still applies.

### Value helper

`Value::vector(&[f32])` is the little-endian blob a `float[N]` parameter
expects; `vector_with_dim` refuses a length that is not `dim` before the
driver sees it.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(vec0_table_test) + binary(vec0_value_test)'
cargo nextest run -p toolu-orm-macros -E 'binary(vec0_macro_test)'
cargo nextest run -p toolu-orm-cli -E 'binary(vec0_loop_sqlite_test)'
cargo nextest run -p toolu-orm-connection --features rusqlite -E 'binary(vec0_value_rusqlite_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | vec0_table_test | ddl::sqlite_ddl_creates_the_virtual_table |
| default | vec0_table_test | ddl::sqlite_ddl_omits_types_strict_and_constraints |
| default | vec0_table_test | ddl::postgres_reports_the_skipped_table_instead_of_emitting_ddl |
| default | vec0_table_test | ddl::a_vector_column_on_an_ordinary_table_is_a_byte_column |
| default | vec0_table_test | rendering::builder_renders_one_argument_per_column_in_declaration_order |
| default | vec0_table_test | rendering::a_vector_without_a_metric_omits_the_distance_metric |
| default | vec0_table_test | rendering::the_vector_column_carries_its_element_type_and_dimension |
| default | vec0_table_test | rendering::build_prevalidated_matches_build_for_a_valid_table |
| default | vec0_table_test | refusals::a_name_vec0_cannot_read_is_refused_instead_of_quoted |
| default | vec0_table_test | refusals::an_identifier_must_start_with_a_letter |
| default | vec0_table_test | refusals::a_bit_vector_cannot_carry_a_distance_metric |
| default | vec0_table_test | snapshot_round_trip::snapshot_round_trip_keeps_the_dimension_and_the_arguments |
| default | vec0_value_test | an_embedding_becomes_its_little_endian_f32_bytes |
| default | vec0_value_test | the_blob_is_four_bytes_per_dimension |
| default | vec0_value_test | an_empty_embedding_is_an_empty_blob |
| default | vec0_value_test | a_matching_length_passes_the_dimension_check |
| default | vec0_value_test | a_mismatched_length_is_refused_with_both_numbers |
| default | vec0_macro_test | table_def_matches_the_hand_built_vec0_table |
| default | vec0_macro_test | table_def_is_a_virtual_vec0_table |
| default | vec0_macro_test | multiple_vector_columns_are_allowed |
| default | vec0_macro_test | the_vector_column_carries_its_element_type_and_dimension |
| default | vec0_macro_test | the_column_module_is_generated |
| default | vec0_macro_test | the_builder_factories_are_generated |
| default | vec0_loop_sqlite_test | migrating_vec0_without_the_extension_is_a_named_missing_module |
| default | vec0_loop_sqlite_test | an_fts5_only_migration_still_applies_on_the_same_driver |
| default | vec0_loop_sqlite_test | regenerating_the_same_vec0_schema_finds_no_change |
| default | vec0_loop_sqlite_test | changing_the_dimension_is_refused_without_writing_a_migration |
| default | vec0_loop_sqlite_test | changing_the_distance_metric_is_refused_without_writing_a_migration |
| default | vec0_loop_sqlite_test | a_driver_message_without_a_module_name_stays_database |
| rusqlite-only | vec0_value_rusqlite_test | vector_bytes_round_trip_through_a_real_blob_column |
| rusqlite-only | vec0_value_rusqlite_test | a_mismatched_length_is_refused_before_the_driver |
