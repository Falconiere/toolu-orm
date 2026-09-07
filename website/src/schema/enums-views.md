# Enums and views

## Enums as columns

`#[derive(ColumnEnum)]` implements `EnumSchema` for a unit-variant enum: it
reports the variant names, honouring `#[serde(rename_all = "…")]`.

```rust
use toolu_orm_macros::{table, ColumnEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ColumnEnum)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
  Pending,
  Running,
  Success,
  Failed,
}
```

Use the enum directly as a field type on a table. The column becomes `TEXT` and
the generator adds a `CHECK` constraint listing the variants:

```rust
#[table(name = "pipeline_runs")]
pub struct PipelineRun {
  #[column(primary_key)]
  pub id: Uuid,
  #[column(not_null, default = "'pending'")]
  pub status: RunStatus,
}
```

```sql
"status" TEXT NOT NULL DEFAULT 'pending' CHECK("status" IN ('pending', 'running', 'success', 'failed'))
```

Add a variant and the next `run_generate` sees a changed `CHECK` and writes the
migration for it. Variants must be unit variants — a variant with fields is a
compile error from the derive.

`#[column(as_text)]` is the escape hatch for any other type you want stored as
`TEXT` without a `CHECK`.

## Views

`#[view]` generates a plain Rust struct holding a subset of the table's fields,
with the marker types mapped to real Rust types. Use it for the read shapes that
do not need every column.

```rust
#[table(name = "users")]
#[view(UserPublic, pick(id, email))]
#[view(UserWithoutSecrets, omit(password_hash))]
pub struct UsersTable {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub email: Text,
  #[column(not_null)]
  pub password_hash: Text,
  #[column(not_null)]
  pub created_at: Integer,
}
```

`pick(...)` keeps exactly the listed fields; `omit(...)` keeps everything else.
Both attributes are repeatable, and the generated structs are emitted next to the
table struct with the same visibility.

The view struct is a data shape, not a query: pair it with a hand-written or
derived `FromRow` impl and `select_for::<UserPublic>()` to select exactly the
columns it declares. See [Row mapping](row-mapping.md).
