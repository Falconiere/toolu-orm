# Defining tables

`#[table]` takes a struct of marker types and produces the schema metadata, the
typed columns, and the builder factories for that table.

```rust
use toolu_orm_core::column::{Integer, Text, Timestamp, Uuid};
use toolu_orm_macros::table;

#[table(name = "pipeline_runs", strict = true)]
#[index("idx_runs_pipeline", pipeline_id)]
#[unique_index("uq_runs_external_id", external_id)]
pub struct PipelineRun {
  #[column(primary_key, default = "uuid4_str()")]
  pub id: Uuid,
  #[column(not_null, references = "pipelines(id)", on_delete = "cascade")]
  pub pipeline_id: Uuid,
  #[column(not_null)]
  pub external_id: Text,
  #[column(not_null, default = "'pending'")]
  pub status: Text,
  #[column(not_null, default = "datetime('now')")]
  pub created_at: Timestamp,
  pub finished_at: Timestamp,
  pub attempts: Integer,
}
```

The struct itself keeps its fields (the marker types are stripped of their
`#[column]` attributes); everything else is generated next to it.

## What the macro generates

| Item | Shape |
|---|---|
| `PipelineRun::table_def()` | `TableDef { name, columns, indexes, strict }` — the `TableSchema` impl. |
| `mod pipeline_runs` | Named after the **table**, holds `TABLE`, `ALL_COLUMNS` and one `Column<T>` per field. |
| `PipelineRun::select()` | `SelectBuilder` bound to the table, no column list. |
| `PipelineRun::select_for::<T>()` | `SelectBuilder` with `columns_raw(T::REQUIRED_COLUMNS)`. |
| `PipelineRun::insert()` / `update()` / `delete()` | The matching builders, bound to the table. |

```rust
pipeline_runs::TABLE;        // "pipeline_runs"
pipeline_runs::ALL_COLUMNS;  // ["id", "pipeline_id", "external_id", …]
pipeline_runs::status;       // Column<Text>
```

## Table attributes

| Attribute | Effect |
|---|---|
| `#[table(name = "…")]` | The SQL table name. Required — it also names the generated column module. |
| `#[table(name = "…", strict = true)]` | Emits a SQLite/Turso `STRICT` table and switches DDL to the STRICT type set. |
| `#[index("name", col_a, col_b)]` | Secondary index. Repeatable. |
| `#[unique_index("name", col)]` | Unique index. Repeatable — this is how you express multi-column uniqueness. |
| `#[view(Name, pick(a, b))]` / `#[view(Name, omit(c))]` | Generates a subset struct from the table. See [Enums and views](enums-views.md). |

## Column attributes

| Attribute | Effect |
|---|---|
| `#[column(primary_key)]` | `PRIMARY KEY`. |
| `#[column(not_null)]` | `NOT NULL`. Omit it for a nullable column. |
| `#[column(unique)]` | `UNIQUE` on the column itself. |
| `#[column(default = "…")]` | Raw SQL default, e.g. `"unixepoch()"`, `"'pending'"`, `"uuid4_str()"`. Emitted verbatim, translated per dialect where a SQLite-specific form has a Postgres equivalent. |
| `#[column(references = "table(col)")]` | Foreign key. |
| `#[column(on_delete = "…")]` / `#[column(on_update = "…")]` | Referential action: `cascade`, `set_null`, `set_default`, `restrict`, `no_action`. |
| `#[column(as_text)]` | Store the field's type as `TEXT` — used for enums and custom types. |
| `#[column(column_type = "Jsonb")]` | Override the inferred `ColumnType` for that field. |

Attributes combine in one `#[column(...)]`:

```rust
#[column(not_null, unique, references = "orgs(id)", on_delete = "cascade")]
pub org_id: Uuid,
```

## Registering tables

A `SchemaRegistry` is the input to migration generation. It sorts tables by name,
so the generated SQL is stable across runs.

```rust
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::table::TableSchema;

let registry = SchemaRegistry::from_tables(vec![
  OrgsTable::table_def(),
  UsersTable::table_def(),
  PipelineRun::table_def(),
]);

registry.find_table("users");  // Option<&TableDef>
registry.tables();             // &[TableDef]
```

Everything downstream — the diff, the migration file, the snapshot — reads this
registry. A table missing from it is a table the generator will try to drop.
