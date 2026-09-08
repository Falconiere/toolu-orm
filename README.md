<div align="center">

# 🗄️ toolu-orm

### Schema-first Rust ORM — one struct, three databases, migrations you can read.

Define a table once as a Rust struct. Get a typed query builder, a `FromRow`
mapper, and a schema snapshot back. Diff the snapshot into a plain-SQL
migration, apply it with a SHA-256-checked journal, and run the same code
against **libsql** (local file, in-memory, or Turso), **rusqlite**, or
**Postgres**. No runtime reflection, no macro-generated SQL you can't read.

[![crates.io](https://img.shields.io/crates/v/toolu-orm-core?style=flat-square&color=blue)](https://crates.io/crates/toolu-orm-core)
[![docs.rs](https://img.shields.io/docsrs/toolu-orm-core?style=flat-square)](https://docs.rs/toolu-orm-core)
[![CI](https://img.shields.io/github/actions/workflow/status/Falconiere/toolu-orm/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/Falconiere/toolu-orm/actions/workflows/ci.yml)
[![Docs](https://img.shields.io/badge/docs-toolu--orm-c8ff4d.svg?style=flat-square)](https://falconiere.github.io/toolu-orm/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg?style=flat-square)](LICENSE)
[![Rust 1.94](https://img.shields.io/badge/rust-1.94-orange.svg?style=flat-square)](rust-toolchain.toml)
[![No unwrap](https://img.shields.io/badge/src-no%20unwrap%20%C2%B7%20no%20panic-purple.svg?style=flat-square)](#contributing)

**[Documentation](https://falconiere.github.io/toolu-orm/)** · [Why](#why-toolu-orm) · [Features](#features) · [How it works](#how-it-works) · [Install](#install) · [Quickstart](#quickstart) · [Tables](#defining-tables) · [Queries](#query-builders) · [Relations](#relations) · [Migrations](#migrations) · [Drivers](#drivers) · [Contributing](#contributing)

</div>

---

## Why toolu-orm?

Most Rust database layers make you pick a side:

- **Query builders** give you type-safe SQL but leave schema evolution to you.
  The migration folder drifts from the structs, and nobody notices until prod.
- **Full ORMs** own the schema but hide the SQL behind a runtime, a DSL, or a
  code generator you have to re-run and re-learn.

**toolu-orm keeps the struct as the single source of truth and generates
everything else from it.** `#[table]` produces a `TableDef`. A `SchemaRegistry`
of those defs is diffed against the last JSON snapshot to write the next
`NNNN_name.sql` migration — plain SQL you can read in review. A journal records
each file's SHA-256 so a migration edited after it shipped fails loudly instead
of silently diverging.

The same struct also hands you typed `Column<T>` constants, `select()` /
`insert()` / `update()` / `delete()` builder factories, and an async executor
that speaks `?1` to SQLite and `$1` to Postgres. Swap the driver by flipping a
Cargo feature; the application code does not change.

> Extracted from a production backend where it drives Turso embedded replicas
> in the field and Postgres in the cloud, from one set of table structs.

---

## Features

| | |
|---|---|
| 🧱 **Schema as code** | `#[table]` turns a struct into a `TableDef` with primary keys, defaults, foreign keys with `on_delete` / `on_update`, `strict` tables, and `#[index]` / `#[unique_index]`. |
| 🔁 **Diff-driven migrations** | `run_generate` diffs your registry against the last `*.snapshot.json` and writes numbered SQL with a `--> statement-breakpoint` separator. `run_migrate` applies pending files in one transaction each; `get_status` lists applied and pending. |
| 🔐 **Tamper-evident journal** | `_journal.json` stores a `sha256:` hash per migration. A file that changed after it was recorded stops the run with `MigrateError::HashMismatch`. |
| 🧮 **Typed columns, typed expressions** | Generated `Column<T>` constants (`users::email`) build `Expr` trees: `eq` / `ne` / `in_list` / `not_in` / `is_null` on every column, `like` on text, `gt` / `lt` / `gte` / `lte` / `between` on numbers, combined with `.and()` / `.or()`. Table-qualified, always quoted. |
| 🏗️ **Four builders, one executor** | `SelectBuilder`, `InsertBuilder` (with `or_ignore` / `or_replace`), `UpdateBuilder` (`set` / `set_expr`), `DeleteBuilder`. All share `.execute()`; select adds `fetch_all`, `fetch_one`, `fetch_optional`, `count`, `exists`. |
| 🌐 **Dialect-aware SQL** | `to_sql_for(Dialect::Sqlite)` emits `?N` placeholders; `Dialect::Postgres` emits `$N`, `ON CONFLICT ... DO UPDATE SET ... = EXCLUDED`, and `LEFT JOIN LATERAL` + `json_agg` for relations. |
| 🕸️ **Relational loads without N+1** | `#[derive(Relational)]` with `#[has_many]`, `#[belongs_to]`, `#[many_to_many]`; `RelationalQuery` fetches parent + children as JSON arrays in a single statement per dialect. |
| 🔎 **Full-text search** | `#[fts5_table]` (or the `Fts5Table` builder) declares an SQLite FTS5 virtual table with `UNINDEXED` columns, a free-form tokenizer, and external content. Migrations emit `CREATE VIRTUAL TABLE ... USING fts5(...)`. |
| 🧭 **Vector tables** | `#[vec0_table]` (or `Vec0Table`) declares an sqlite-vec `vec0` virtual table with a typed `Vector { dim, element }` column, partition keys, metadata and auxiliary columns. `Value::vector` encodes little-endian f32 embeddings. Load `sqlite-vec` on the connection before `run_migrate` (#12); otherwise migrate fails as `MissingExtension`. |
| 🧬 **Enums and views** | `#[derive(ColumnEnum)]` stores a Rust enum as text; `#[view(Name, pick(...))]` / `omit(...)` generates subset structs from a table. |
| 🔄 **Transactions** | `conn.run_transaction(|tx| async move { ... })` commits on `Ok`, rolls back on `Err`. |
| 🔌 **Three drivers, one trait** | `DbConnection` over libsql (async, Turso embedded replica with sync retry), rusqlite (sync, wrapped in `spawn_blocking`), and Postgres (`deadpool-postgres` pool, rustls TLS). |
| 🛡️ **Panic-free `src/`** | Workspace-wide `clippy::unwrap_used`, `expect_used`, `panic`, `indexing_slicing` are `deny`. No `#[allow]` anywhere. |

---

## How it works

Five crates. `orm-core` is the foundation; every other crate depends on it, and
only `orm-cli` depends on `orm-connection`.

```
                 ┌────────────────────────┐
                 │      toolu-orm-core     │  TableDef · ColumnType · Value · Expr
                 │  schema · snapshot ·    │  Column<T> · Snapshot · Journal
                 │  diff · dialect · row   │  Dialect { Sqlite, Postgres }
                 └───────────┬────────────┘
        ┌────────────────────┼─────────────────────┐
        ▼                    ▼                     ▼
┌───────────────┐   ┌────────────────┐   ┌──────────────────────┐
│ toolu-orm-    │   │ toolu-orm-     │   │ toolu-orm-connection │
│ macros        │   │ query          │   │ Database · DbConnection
│ #[table]      │   │ Select/Insert/ │   │ libsql · rusqlite ·   │
│ FromRow       │   │ Update/Delete  │   │ PgDatabase (pool+TLS) │
│ Relational    │   │ RelationalQuery│   └──────────┬───────────┘
│ ColumnEnum    │   │ Executor · tx  │              │
└───────────────┘   └────────────────┘              ▼
                                          ┌──────────────────────┐
                                          │    toolu-orm-cli      │
                                          │ run_generate          │
                                          │ run_migrate           │
                                          │ get_status            │
                                          └──────────────────────┘
```

The migration loop in one line:

```
structs ──#[table]──▶ TableDef ──SchemaRegistry──▶ diff vs last snapshot ──▶ NNNN_name.sql + snapshot.json + _journal.json
```

---

## Install

The `toolu-orm` facade pulls in the whole stack behind one version and one
feature list:

```toml
[dependencies]
toolu-orm = { version = "0.1", features = ["libsql"] }
tokio     = { version = "1", features = ["rt-multi-thread", "macros"] }
```

That is the whole list. It re-exports `toolu_orm::core`, `toolu_orm::query`,
`toolu_orm::connection` and the proc macros, and the macros expand to absolute
paths resolved against your `Cargo.toml`, so `#[table]` and the derives work
with no other dependency and no import beyond the macro itself. `toolu_orm::prelude`
still exists as a convenience glob; nothing requires it.

`toolu-orm-cli` is not re-exported by the facade. Add it as a normal
dependency when you generate or apply migrations from your own binary — it is a
library crate with no `[[bin]]` of its own:

```toml
toolu-orm-cli = { version = "0.1", default-features = false, features = ["libsql"] }
```

If you do name `toolu-orm-core` directly as well, keep it on the same version as
`toolu-orm`: they share one workspace version, and a mismatch means two
different `toolu_orm_core` crates in the graph, whose types do not interoperate.

### Depending on the crates directly

Every crate exposes the same driver features (`libsql`, `rusqlite`, `postgres`)
and forwards them to `toolu-orm-core`. **Enable the drivers you need on every
crate you depend on** so Cargo unifies them into one shape.

```toml
[dependencies]
toolu-orm-core       = { version = "0.1", default-features = false, features = ["libsql"] }
toolu-orm-macros     = { version = "0.1", features = ["libsql"] }
toolu-orm-query      = { version = "0.1", features = ["libsql"] }
toolu-orm-connection = { version = "0.1", features = ["libsql"] }
toolu-orm-cli        = { version = "0.1", default-features = false, features = ["libsql"] }
tokio                = { version = "1", features = ["rt-multi-thread", "macros"] }
```

For Postgres, replace `"libsql"` with `"postgres"`. `toolu-orm-core` and
`toolu-orm-cli` default to `libsql`; the other crates have no default driver.

---

## Quickstart

Define a table, generate and apply a migration, insert, and read back — against
an in-memory libsql database.

```toml
[dependencies]
toolu-orm     = { version = "0.1", features = ["libsql"] }
toolu-orm-cli = { version = "0.1", default-features = false, features = ["libsql"] }
tokio         = { version = "1", features = ["rt-multi-thread", "macros"] }
```

`toolu-orm-cli` is a plain library crate despite the name — it ships no
binary, so `run_generate`, `run_migrate` and `get_status` are called from your
own code (see [Migrations](#migrations) for the usual `bin/migrate.rs`).

```rust
use toolu_orm::connection::Database;
use toolu_orm::core::column::{Integer, Text};
use toolu_orm::core::dialect::Dialect;
use toolu_orm::core::query_column::CommonOps;
use toolu_orm::core::schema::SchemaRegistry;
use toolu_orm::core::table::TableSchema;
use toolu_orm::{table, FromRow};

#[table(name = "users")]
pub struct UsersTable {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub email: Text,
  #[column(not_null, default = "unixepoch()")]
  pub created_at: Integer,
}

// The derive follows the drivers active on `toolu-orm-core`: one driver
// (libsql here) means a single `from_row`. See "Row mapping".
#[derive(FromRow)]
pub struct User {
  pub id: String,
  pub email: String,
  pub created_at: i64,
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
  let exec = conn.inner_conn();   // &libsql::Connection — borrows `conn`, so
                                  // keep `conn` alive for as long as `exec` is used

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

`#[table]` generated everything used above: the `users` companion module with
one `Column<T>` per field, `UsersTable::table_def()`, and the `select()` /
`select_for::<T>()` / `insert()` / `update()` / `delete()` factories.

Two connection surfaces show up there. `DbConnection` — what `db.connect()`
returns — is what `run_migrate` and `get_status` take. `Executor` is what the
query builders run on, and it is implemented for the **driver's own** connection
type (`libsql::Connection`, `rusqlite::Connection`, `tokio_postgres::Client`,
`PgTransaction`), which `conn.inner_conn()` hands out.

---

## Defining tables

```rust
use toolu_orm_macros::{table, ColumnEnum};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ColumnEnum)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus { Pending, Running, Success, Failed }

#[table(name = "pipeline_runs", strict = true)]
#[index("idx_runs_pipeline", pipeline_id)]
#[index("idx_runs_status", status)]
pub struct PipelineRun {
  #[column(primary_key, default = "uuid4_str()")]
  pub id: Uuid,
  #[column(not_null, references = "pipelines(id)", on_delete = "cascade")]
  pub pipeline_id: Uuid,
  #[column(not_null, default = "'pending'")]
  pub status: RunStatus,
  #[column(not_null, default = "datetime('now')")]
  pub created_at: Timestamp,
}
```

| Attribute | Effect |
|---|---|
| `#[table(name = "...", strict = true)]` | Table name; `strict` switches column SQL types to the SQLite / Turso `STRICT` set. |
| `#[column(primary_key)]` | Primary key. |
| `#[column(not_null)]` | `NOT NULL`; omit it for a nullable column. |
| `#[column(default = "...")]` | Raw SQL default, e.g. `"unixepoch()"`, `"'pending'"`, `"uuid4_str()"`. |
| `#[column(references = "t(col)", on_delete = "cascade", on_update = "...")]` | Foreign key with referential actions. |
| `#[column(as_text)]` | Store an enum or custom type as `TEXT`. |
| `#[index("name", col, ...)]` / `#[unique_index("name", col)]` | Secondary indexes on the table; `unique_index` is how you express uniqueness. |
| `#[view(Name, pick(a, b))]` / `#[view(Name, omit(c))]` | Generate a subset struct from the table. |

Field types map to `ColumnType`: `Text`, `Integer`, `Real`, `Blob`, `Uuid`,
`Boolean`, `Timestamp`, `Date`, `Time`, `Json`, plus Postgres-flavoured
`BigInt`, `SmallInt`, `Varchar(n)`, `Serial`, `BigSerial`, `Jsonb`, `Numeric`,
`Char(n)`, `Array`.

**Row mapping.** `#[derive(FromRow)]` fills `REQUIRED_COLUMNS` from the field
names in declaration order and reads each field positionally at its own index, so
`select_for::<T>()` picks exactly the columns `T` needs, in the order it decodes
them. An `Option<T>` field decodes SQL `NULL` as `None`; anything else missing
or undecodable is a `DbCoreError::RowMapping` naming the column's index and
name, never a panic.

The derive expands to whichever shape the drivers on `toolu-orm-core` gave the
trait, so it compiles on every combination — one driver means a single
`from_row`, two or more mean one method per driver:

| Drivers active | Methods the derive implements |
|---|---|
| one of `libsql` / `rusqlite` / `postgres` | `from_row` |
| any two | two of `from_pg_row` / `from_libsql_row` / `from_rusqlite_row` |
| all three | all three |

`#[from_row(with = "f")]` on a field routes the decoded value through `f`
(`FieldTy -> Result<FieldTy, E>`) to normalize or reject it.

Writing the impl by hand stays supported, and is the way out when a field type
the active driver cannot decode needs a conversion:

```rust
use toolu_orm_core::{error::DbCoreError, row::FromRow};

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
```

### Virtual tables (SQLite FTS5)

`#[fts5_table]` declares a full-text index. It generates the same items as
`#[table]` — a `TableSchema`, typed `Column<T>` constants, builder factories —
but the `TableDef` carries `TableKind::Virtual { module: "fts5", args }`:

```rust
use toolu_orm_macros::fts5_table;

#[fts5_table(name = "memory_fts", tokenize = "porter unicode61 remove_diacritics 2")]
pub struct MemoryFts {
  #[column(unindexed)]
  pub memory_id: Text,
  pub body: Text,
  pub tags: Text,
}
```

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS "memory_fts" USING fts5("memory_id" UNINDEXED,
  "body", "tags", tokenize = 'porter unicode61 remove_diacritics 2');
```

`name` is required; `tokenize`, `prefix`, `content`, `content_rowid`,
`columnsize` and `detail` are passed through to the module verbatim, so a
tokenizer this crate has never heard of still works. `#[column(unindexed)]`
appends `UNINDEXED`: the value is stored and readable but not searchable.
`Fts5Table` is the same thing without the macro, for a `TableDef` built at
runtime.

SQLite cannot `ALTER` a virtual table, so `run_generate` refuses any in-place
change to one — a new column, a different tokenizer, a switched module — with
`DbCoreError::VirtualTableChange` and writes no migration; drop and recreate it
instead. Creating, dropping and renaming work as usual. On Postgres the table is
skipped with a comment naming it. Other modules (`rtree`, …) can still use
`TableKind::virtual_table(module, args)` directly; `vec0` has its own builder
and macro below.

### Virtual tables (sqlite-vec `vec0`)

`#[vec0_table]` declares a vector index the same way `#[fts5_table]` declares a
full-text one. The `TableDef` carries `TableKind::Virtual { module: "vec0", args }`
and every vector column is `ColumnType::Vector { element, dim }`:

```rust
use toolu_orm_macros::vec0_table;
use toolu_orm_core::column::{Integer, Text, Vector};

#[vec0_table(name = "memory_vec")]
pub struct MemoryVec {
  #[column(primary_key)]
  pub memory_id: Text,
  #[column(dim = 1024, distance_metric = "cosine")]
  pub embedding: Vector,
  #[column(partition_key)]
  pub user_id: Integer,
  pub label: Text,
  #[column(auxiliary)]
  pub contents: Text,
}
```

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS "memory_vec" USING "vec0"(
  memory_id text primary key,
  embedding float[1024] distance_metric=cosine,
  user_id integer partition key,
  label text,
  +contents text
);
```

`name` is required. A `Vector` field needs `#[column(dim = N)]` and may take
`element` (`"float"` / `"int8"` / `"bit"`) and `distance_metric`
(`"l2"` / `"cosine"` / `"l1"`). Fields with `partition_key` or `auxiliary` map
to those `vec0` roles; everything else is metadata. `vec0` cannot quote
identifiers, so a hostile name fails the build instead of being escaped.
`Value::vector(&[f32])` (and `vector_with_dim`) produce the little-endian blob
a `float[N]` parameter expects.

Unlike FTS5, `vec0` is **not** built into SQLite. Register `sqlite-vec` on the
connection (typically via `sqlite3_auto_extension` / the bindings' equivalent)
**before** `run_migrate`. A migration that creates a `vec0` table against a
connection that never loaded it fails as
`MigrateError::MissingExtension { module: "vec0", … }`. Connection setup
ordering is issue #12 (`from_connection`). Changing `dim`, the element type, or
`distance_metric` is refused by the diff the same way FTS5 changes are — drop,
recreate, and re-embed in a hand-written migration.

---

## Query builders

The snippets below name the crates directly (`toolu_orm_core::…`,
`toolu_orm_query::…`); through the facade the same items are
`toolu_orm::core::…` and `toolu_orm::query::…`.

Every builder renders with `to_sql()` (current dialect) or
`to_sql_for(Dialect::…)` and returns `(String, Vec<Value>)`. Column references
are always table-qualified and quoted. The comparison methods come from three
traits in `toolu_orm_core::query_column`: `CommonOps` (`eq`, `ne`, `in_list`,
`not_in`, `is_null`, `is_not_null`), `TextOps` (`like`), and `NumericOps`
(`gt`, `lt`, `gte`, `lte`, `between`).

```rust
use toolu_orm_core::query_column::{CommonOps, NumericOps};
use toolu_orm_query::{delete::DeleteBuilder, insert::InsertBuilder, select::SelectBuilder, update::UpdateBuilder};

// SELECT with filters, join, ordering, paging
let (sql, params) = SelectBuilder::new("users")
  .columns_raw(&["id", "email"])
  .filter(users::org_id.eq("org123"))
  .filter(users::created_at.gt(0i32))
  .join("pipelines", users::id.equals(&pipelines::user_id))   // INNER JOIN; left_join() too
  .order_by(users::created_at.desc())
  .limit(10)
  .offset(0)
  .to_sql_for(Dialect::Sqlite);
// SELECT "id", "email" FROM "users"
//   INNER JOIN "pipelines" ON "users"."id" = "pipelines"."user_id"
//   WHERE "users"."org_id" = ?1 AND "users"."created_at" > ?2
//   ORDER BY "users"."created_at" DESC LIMIT ?3 OFFSET ?4

// INSERT, with upsert flavours that render per dialect
InsertBuilder::new("seeds").or_ignore().set(&seeds::id, "seed-1").to_sql_for(Dialect::Postgres);
// INSERT INTO "seeds" ("id") VALUES ($1) ON CONFLICT DO NOTHING
InsertBuilder::new("run_status").or_replace().set(&run_status::run_id, "run-1").set(&run_status::status, "running")
  .to_sql_for(Dialect::Postgres);
// ... ON CONFLICT ("run_id") DO UPDATE SET "status" = EXCLUDED."status"

// UPDATE with a bound value and a raw SQL expression
UpdateBuilder::new("users").set(&users::email, "new@example.com").set_expr(&users::updated_at, "unixepoch()")
  .filter(users::id.eq("user-1")).to_sql_for(Dialect::Sqlite);
// UPDATE "users" SET "email" = ?1, "updated_at" = unixepoch() WHERE "users"."id" = ?2

// DELETE
DeleteBuilder::new("users").filter(users::id.eq("user-1")).to_sql_for(Dialect::Postgres);
// DELETE FROM "users" WHERE "users"."id" = $1
```

**Executing.** All four builders share `.execute(exec) -> u64`, where `exec` is
the driver connection (`&libsql::Connection`, `&rusqlite::Connection`,
`&tokio_postgres::Client`, or a transaction) — not the `DbConnection` wrapper.
Select adds:

```rust
let users: Vec<User> = SelectBuilder::new("users").columns_raw(&["id", "email", "created_at"]).fetch_all(exec).await?;
let one: User        = UsersTable::select_for::<User>().filter(users::id.eq("u_1")).fetch_one(exec).await?;   // QueryError::NotFound if empty
let maybe: Option<User> = UsersTable::select_for::<User>().filter(users::id.eq("nope")).fetch_optional(exec).await?;
let n: i64           = UsersTable::select().count(exec).await?;
let any: bool        = UsersTable::select().exists(exec).await?;
```

On the `rusqlite` driver these are synchronous: same names, no `.await`.

Also available: `to_count_sql_for`, `to_exists_sql_for`,
`SelectBuilder::raw().column_expr(expr, alias)`, and
`columns_typed(&[&dyn ColumnRef])`.

**Transactions.** Anything that errors inside the closure rolls the whole block back.

```rust
use toolu_orm_query::transaction::TransactionExt;

// `conn` is a libsql::Connection — TransactionExt is implemented on the driver type.
conn.run_transaction(|tx| async move {
  InsertBuilder::new("users").set(&users::id, "tx1").set(&users::email, "tx@example.com").execute(&tx).await?;
  UpdateBuilder::new("counters").set_expr(&counters::users, "users + 1").execute(&tx).await?;
  Ok(())
}).await?;
```

---

## Relations

Declare the shape you want back; the query is one statement per dialect.

```rust
use toolu_orm_macros::Relational;

#[derive(Relational)]
#[relational(table = "users")]
struct UserWithPosts {
  pub id: String,
  pub name: String,
  #[has_many(table = "posts", foreign_key = "author_id", columns = ["id", "title"])]
  pub posts: Vec<PostRow>,
}
// #[belongs_to(...)]    → field is Option<T>, same keys
// #[many_to_many(...)]  → adds through = "post_tags", local_key = "post_id"
```

The typed builder tracks the result tuple at compile time:

```rust
use toolu_orm_query::relational_builder::RelationalQuery;

let q = RelationalQuery::<(UserRow,)>::new("users", &["id", "name"])
  .with_many::<PostRow>("posts", "posts", "id", "author_id", &["id", "title"])
  .with_one::<ProfileRow>("profile", "profiles", "id", "user_id", &["id", "bio"]);
let _: RelationalQuery<(UserRow, Vec<PostRow>, Option<ProfileRow>)> = q;
```

- **SQLite** renders correlated subqueries with `json_group_array`.
- **Postgres** renders `LEFT JOIN LATERAL` with `json_agg` / `json_build_array`.

An untyped `RelationalSelectBuilder` exposes the same `with_many` / `with_one`
plus `to_sql_sqlite()` / `to_sql_postgres()` when you only want the SQL.

---

## Migrations

```rust
use toolu_orm_cli::{generate::run_generate, status::get_status};
use toolu_orm_cli::migrate::{run_migrate, run_migrate_embedded};

let wrote = run_generate(&registry, "migrations", "add_posts", Dialect::Postgres)?;
//  → Some("0002_add_posts.sql"), or None when the schema did not change

let applied = run_migrate(&conn, "migrations", Dialect::Postgres).await?;   // u32 files applied
// …or, with the SQL compiled into the binary:
let applied = run_migrate_embedded(&conn, MIGRATIONS, Dialect::Postgres).await?;

let status = get_status(&conn, "migrations", Dialect::Postgres).await?;
println!("applied: {:?}, pending: {:?}", status.applied, status.pending);
```

What lands on disk:

```
migrations/
├── _journal.json                # order + "sha256:…" per file
├── 0001_init.sql
├── 0001_init.snapshot.json      # schema state after this migration
├── 0002_add_posts.sql
└── 0002_add_posts.snapshot.json
```

- Files apply in name order; the applied set is tracked in a `_migrations`
  table on the target database.
- A file holding several statements separates them with
  `--> statement-breakpoint`. Each file runs inside `BEGIN` / `COMMIT`.
- The journal hash is verified before a file runs. Edit a shipped migration
  and `run_migrate` stops with `MigrateError::HashMismatch`.
- Shipping a single binary with no migrations directory on the target machine?
  Bake the SQL in with `include_str!` and apply it with
  `run_migrate_embedded(&conn, MIGRATIONS, dialect)`, where `MIGRATIONS` is a
  `&[EmbeddedMigration]` of `name` / `sql` / `hash`. Same hashes, same
  one-transaction-per-migration, and a database is free to move between the two
  sources.
- Adopting toolu-orm on a database that already has the schema? Baseline it with
  `mark_applied(&conn, "migrations", &["0001_init.sql"], dialect)` — or
  `mark_applied_through(&conn, "migrations", "0016_add_tags.sql", dialect)` — to
  record those journal entries (with their journal hashes) without executing
  them, so the next `run_migrate` starts from the first one you did not baseline.
- Snapshots are plain JSON (`version`, `dialect`, `id`, `prev_id`, `tables`,
  `enums`, `meta`), so a schema diff is reviewable in the PR alongside the SQL.

---

## Drivers

| Feature | Backing crate | Mode | Open with |
|---|---|---|---|
| `libsql` | [libsql](https://crates.io/crates/libsql) | async; local file, `:memory:`, or Turso embedded replica | `Database::init_local(path)` · `Database::init_remote(RemoteConfig)` |
| `rusqlite` | [rusqlite](https://crates.io/crates/rusqlite) (bundled) | sync natively (`DbConnectionBlocking`), wrapped in `spawn_blocking` for the async `DbConnection` | `RusqliteConnection::from_connection(conn)` (no runtime) · `::open(path)` · `::open_in_memory()` |
| `postgres` | [tokio-postgres](https://crates.io/crates/tokio-postgres) + [deadpool](https://crates.io/crates/deadpool-postgres) | async pool, rustls TLS | `PgDatabase::init(&PgConfig)` then `.connect()` |

```rust
// Turso embedded replica: local file kept in sync with the remote
let db = Database::init_remote(RemoteConfig {
  replica_path: "data/app.db".into(),
  url: "libsql://my-db.turso.io".into(),
  auth_token: std::env::var("TURSO_TOKEN")?,
  sync_interval_secs: 5,
  max_sync_attempts: 5,
}).await?;

// Postgres pool
let pg = PgDatabase::init(&PgConfig {
  host: "localhost".into(), port: 5432,
  user: "app".into(), password: std::env::var("PGPASSWORD")?, dbname: "app".into(),
  max_connections: 10, ssl: true,
}).await?;
let conn = pg.connect().await?;
```

All backends implement `DbConnection` (`execute_sql`, `query_map<T: FromRow>`,
`execute_batch`), which is what `run_migrate` and `get_status` accept.

---

## Contributing

Read **[CLAUDE.md](CLAUDE.md)** first: it holds the workspace map and the
binding rules. The short version:

1. No `.unwrap()`, `.expect()`, `panic!`, `unreachable!`, or `[]` indexing in
   `src/`. Propagate with `?` / `ok_or`. Tests may.
2. No `#[allow]` / `#[expect]`. Fix the warning.
3. No `#[cfg(test)]` in `src/`; tests live in each crate's `tests/`.
4. ≤ 250 lines per file. Over that, split into a folder module whose `mod.rs`
   holds only `mod`, `pub use`, and `//!` docs.
5. One concern per file. No `utils.rs` / `helpers.rs` / `common.rs`.
6. `cargo nextest run`, never `cargo test`.

The quality gate is what CI runs: four feature lanes plus a docs check. Every
test executes against a real database (in-memory libsql, in-memory rusqlite,
or a live Postgres), so start the test Postgres first:

```sh
docker compose -f docker-compose.test.yaml up -d --wait   # postgres:16 on localhost:5434
export TEST_DB_PORT=5434                                   # for_test() defaults to 5433

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo clippy -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres --all-targets -- -D warnings
cargo nextest run -p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres
cargo clippy -p toolu-orm-query --features libsql --all-targets -- -D warnings
cargo nextest run -p toolu-orm-query --features libsql
cargo clippy -p toolu-orm-query --features rusqlite --all-targets -- -D warnings
cargo nextest run -p toolu-orm-query --features rusqlite
cargo clippy -p toolu-orm-connection --features rusqlite --all-targets -- -D warnings
cargo nextest run -p toolu-orm-connection --features rusqlite
bash scripts/check-derive-matrix.sh
bash scripts/check-scenario-docs.sh
```

`TEST_DB_HOST`, `TEST_DB_PORT`, `TEST_DB_USER`, and `TEST_DB_PASSWORD` point the
Postgres suites at another server. Each feature scenario is documented in
[`docs/scenarios/`](docs/scenarios/README.md) with the tests that prove it on
every driver; the docs check fails when a page and its tests drift apart, so
update the page with the test. Use a
[Conventional Commits](https://www.conventionalcommits.org/) subject
(`feat(query): add fetch_optional`).

## Releases

Releases are automated with [release-plz](https://release-plz.ieni.dev). You do
not bump versions or tag by hand.

- Merge Conventional Commits to `main`. release-plz maintains one **release PR**
  that bumps the shared workspace version and rewrites `CHANGELOG.md`.
- Merge that PR to cut the release: the five crates publish to crates.io in
  dependency order, then a single `vX.Y.Z` tag and GitHub Release are created.

## License

[MIT](LICENSE) © Falconiere Barbosa

<div align="center">
<sub>Built in Rust 🦀 · schema is the source of truth · migrations you can read · libsql · rusqlite · Postgres</sub>
</div>
