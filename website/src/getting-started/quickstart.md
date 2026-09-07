# Quickstart

Define a table, generate and apply a migration, write a row and read it back —
against an in-memory libsql database.

```toml
[dependencies]
toolu-orm      = { version = "0.1", features = ["libsql"] }
toolu-orm-core = { version = "0.1", default-features = false, features = ["libsql"] }
toolu-orm-cli  = { version = "0.1", default-features = false, features = ["libsql"] }
tokio          = { version = "1", features = ["rt-multi-thread", "macros"] }
```

`toolu-orm-core` sits alongside the facade because the generated column module
resolves its paths against the extern prelude — see
[the macro-path caveat](installation.md#the-macro-path-caveat).

```rust
use toolu_orm::connection::Database;
use toolu_orm::core::column::{Integer, Text};
use toolu_orm::core::dialect::Dialect;
use toolu_orm::core::error::DbCoreError;
use toolu_orm::core::query_column::CommonOps;
use toolu_orm::core::row::FromRow;
use toolu_orm::core::schema::SchemaRegistry;
use toolu_orm::core::table::TableSchema;
use toolu_orm::prelude::*;                   // required: the macros expand to
                                             // toolu_orm_core / toolu_orm_query paths

#[table(name = "users")]
pub struct UsersTable {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub email: Text,
  #[column(not_null, default = "unixepoch()")]
  pub created_at: Integer,
}

pub struct User {
  pub id: String,
  pub email: String,
  pub created_at: i64,
}

// One driver is active (libsql), so `FromRow` asks for a single `from_row`.
// See "Row mapping" for the derive and the other driver shapes.
impl FromRow for User {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "email", "created_at"];

  fn from_row(row: &libsql::Row) -> Result<Self, DbCoreError> {
    let col = |i: i32, e: libsql::Error| DbCoreError::RowMapping(format!("col {i}: {e}"));
    Ok(Self {
      id: row.get(0).map_err(|e| col(0, e))?,
      email: row.get(1).map_err(|e| col(1, e))?,
      created_at: row.get(2).map_err(|e| col(2, e))?,
    })
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // 1. Schema → migration file (writes migrations/0001_init.sql + snapshot + journal)
  std::fs::create_dir_all("migrations")?;
  let registry = SchemaRegistry::from_tables(vec![UsersTable::table_def()]);
  toolu_orm_cli::generate::run_generate(&registry, "migrations", "init", Dialect::Sqlite)?;

  // 2. Connect and apply whatever is pending
  let db = Database::init_local(":memory:").await?;
  let conn = db.connect()?;
  let applied = toolu_orm_cli::migrate::run_migrate(&conn, "migrations", Dialect::Sqlite).await?;
  println!("applied {applied} migration(s)");

  // 3. Typed writes and reads through the generated builders.
  //    The builders run on the driver connection, which the wrapper exposes.
  let exec = conn.inner_conn();   // &libsql::Connection

  UsersTable::insert()
    .set(&users::id, "u_1")
    .set(&users::email, "ada@example.com")
    .execute(exec)
    .await?;

  let found: Vec<User> = UsersTable::select_for::<User>()
    .filter(users::email.eq("ada@example.com"))
    .fetch_all(exec)
    .await?;
  println!("{} user(s)", found.len());
  Ok(())
}
```

## What the macro generated

`#[table]` expanded into everything used above:

- a `users` companion module — named after the **table name**, not the struct —
  holding one `Column<T>` constant per field plus `TABLE` and `ALL_COLUMNS`;
- `UsersTable::table_def()`, the `TableDef` a `SchemaRegistry` is built from;
- the builder factories `select()`, `select_for::<T>()`, `insert()`, `update()`
  and `delete()`, each pre-bound to the table name.

`select_for::<User>()` selects exactly `User::REQUIRED_COLUMNS`, so the row
mapper and the column list cannot drift apart.

`use toolu_orm::prelude::*;` is not decoration: the macro expansion names
`toolu_orm_core` and `toolu_orm_query` directly, and only the prelude puts those
crate names in scope for a consumer whose sole dependency is `toolu-orm`. It also
re-exports the driver crate — that is where the `libsql::Row` in the `FromRow`
impl below comes from. See [Installation](installation.md).

Note the two connection types: `LibsqlConnection` (what `db.connect()` returns)
is what migrations take, and `conn.inner_conn()` is the driver connection the
query builders run on. [Connections](../drivers/index.md) covers the split.

## What landed on disk

```text
migrations/
├── _journal.json            # migration order + "sha256:…" per file
├── 0001_init.sql            # plain SQL, reviewable in the PR
└── 0001_init.snapshot.json  # schema state after this migration
```

Run `run_generate` again after changing the struct and it writes
`0002_….sql` with only the difference — or returns `None` when nothing changed.

## Next

- [Defining tables](../schema/tables.md) — the full attribute surface.
- [Select](../queries/select.md) and [Filters](../queries/filters.md) — reading data.
- [The migration loop](../migrations/overview.md) — generate, migrate, status.
- [Connections](../drivers/index.md) — swapping libsql for rusqlite or Postgres.
