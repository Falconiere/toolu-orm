# Enums and views

## Enums as columns

`#[derive(ColumnEnum)]` implements `EnumSchema` for a unit-variant enum. Names
default to lowercase; the supported `#[serde(rename_all = "…")]` forms are
`lowercase`, `snake_case`, `UPPERCASE`, `SCREAMING_SNAKE_CASE` and `kebab-case`.
Per-variant `#[serde(rename = "…")]` and other Serde naming rules are not
implemented by `ColumnEnum`.

```rust
use toolu_orm_core::{column::Uuid, serde};
use toolu_orm_macros::{table, ColumnEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ColumnEnum)]
#[serde(crate = "toolu_orm_core::serde", rename_all = "snake_case")]
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
"status" TEXT NOT NULL DEFAULT ('pending') CHECK("status" IN ('pending', 'running', 'success', 'failed'))
```

This is `TEXT` plus `CHECK` on both dialects, including Postgres. The derive
does not implement row decoding or `Into<Value>`; bind the stored string and
decode a string (or write a custom `FromRow` implementation).

Adding a variant changes the detected `CHECK`, but check-only changes on SQLite
currently render migration comments and require a manual table rebuild. On
Postgres, generated CHECK alteration SQL needs review: inline checks are not
given the column-name constraint identifiers used by the diff, and stored
`CHECK(...)` text is wrapped again. Correct the SQL before applying it. The
generator does not guarantee an executable enum-change migration.

Variants must be unit variants — a variant with fields is a compile error from
the derive.

`#[column(as_text)]` is the escape hatch for any other type you want stored as
`TEXT` without a `CHECK`.

## Views

`#[view]` generates a plain Rust struct holding a subset of the table's fields,
with the marker types mapped to real Rust types. Use it for the read shapes that
do not need every column.

```rust
use toolu_orm_core::column::{Integer, Text};

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

`pick(...)` keeps the named fields in their original table declaration order;
`omit(...)` keeps everything else.
Both attributes are repeatable, and the generated structs are emitted next to the
table struct with the same visibility.

Generated views derive `Debug`, `Clone`, `Serialize` and `Deserialize`, using
the core crate's Serde re-export. They do **not** implement `FromRow`, and the
`#[view]` syntax has no option to add that derive. Write a manual `FromRow` impl
for a generated view before passing it to `select_for::<UserPublic>()`, or
declare a separate row struct with `#[derive(FromRow)]`.

The current view mapping is:

| Declared table field | View field |
|---|---|
| `Integer`, `BigInt`, `Timestamp` | `i64` |
| `SmallInt` | `i16` |
| `Real` | `f64` |
| `Boolean` | `bool` |
| `Json` | `serde_json::Value` |
| `Blob` | `Vec<u8>` |
| Other types, including enums, `Jsonb`, `Serial`, `BigSerial`, `Numeric` | `String` |

Fields without column-level `primary_key` or `not_null` are wrapped in
`Option<T>`. Table-level composite keys do not affect that mapping. The mapping
uses the declared field type, ignoring `column_type` overrides, and is not a
promise that a driver can decode the result directly (for example, Postgres
`TIMESTAMPTZ` is not an `i64`). See [Row mapping](row-mapping.md).
