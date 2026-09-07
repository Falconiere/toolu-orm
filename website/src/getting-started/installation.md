# Installation

## The facade

One dependency pulls in the whole stack behind one version and one feature list:

```toml
[dependencies]
toolu-orm = { version = "0.1", features = ["libsql"] }
tokio     = { version = "1", features = ["rt-multi-thread", "macros"] }
# add toolu-orm-cli too if you generate or apply migrations — see below
```

`toolu-orm` contains no logic — it re-exports the four library crates and the
proc macros:

| Path | Crate | Holds |
|---|---|---|
| `toolu_orm::core` | `toolu-orm-core` | schema, columns, expressions, snapshots, journal, `Dialect` |
| `toolu_orm::query` | `toolu-orm-query` | select / insert / update / delete builders, executor, transactions |
| `toolu_orm::connection` | `toolu-orm-connection` | `DbConnection` and the driver adapters |
| `toolu_orm::{table, FromRow, Relational, ColumnEnum}` | `toolu-orm-macros` | the proc macros |

### Importing the macros

Import the macro you use and nothing else:

```rust
use toolu_orm::core::column::Text;
use toolu_orm::table;

#[table(name = "users")]
pub struct UsersTable {
  #[column(primary_key)]
  pub id: Text,
}
```

The expansions emit **absolute** paths — `::toolu_orm::core::…` and
`::toolu_orm::query::…` when the facade is your dependency,
`::toolu_orm_core::…` when you name the crates directly. `toolu-orm-macros`
reads your `Cargo.toml` (via
[`proc-macro-crate`](https://crates.io/crates/proc-macro-crate)) to pick
between them, honouring a Cargo rename. Nothing has to be in scope for the
generated code to resolve, including inside the companion column module
`#[table]` generates.

`toolu_orm::prelude` still re-exports the macros, `toolu_orm_core`,
`toolu_orm_query` and the active driver crate. It is a convenience for code
that names those crates itself; the macros no longer need it.

### Migrations are a separate crate

The facade does not re-export `toolu-orm-cli`. Add it when you generate or apply
migrations from your own binary — despite the name it is a plain library crate
with no `[[bin]]`, so there is nothing to `cargo install`:

```toml
toolu-orm-cli = { version = "0.1", default-features = false, features = ["libsql"] }
```

## Depending on the crates directly

Skip the facade when you want a subset, or when you would rather name each crate
in `Cargo.toml`. Every crate exposes the same driver features and forwards them
to `toolu-orm-core`:

```toml
[dependencies]
toolu-orm-core       = { version = "0.1", default-features = false, features = ["libsql"] }
toolu-orm-macros     = { version = "0.1", features = ["libsql"] }
toolu-orm-query      = { version = "0.1", features = ["libsql"] }
toolu-orm-connection = { version = "0.1", features = ["libsql"] }
toolu-orm-cli        = { version = "0.1", default-features = false, features = ["libsql"] }
```

Only `toolu-orm-core` and `toolu-orm-macros` are mandatory. The expansions
resolve to `::toolu_orm_core` / `::toolu_orm_query` here, since those are the
direct dependencies.

## Driver features

| Feature | Backing crate | Mode |
|---|---|---|
| `libsql` | [libsql](https://crates.io/crates/libsql) | async; local file, `:memory:`, or a Turso embedded replica |
| `rusqlite` | [rusqlite](https://crates.io/crates/rusqlite) (bundled) | sync, wrapped in `spawn_blocking` |
| `postgres` | [tokio-postgres](https://crates.io/crates/tokio-postgres) + [deadpool-postgres](https://crates.io/crates/deadpool-postgres) | async pool, rustls TLS |

On the facade, one feature drives every re-exported crate. Naming the crates
directly means **enabling the drivers you need on every one of them** — Cargo
unifies features across the graph, and the generated code changes shape with the
active set, so a driver enabled on one crate but not another is a compile error,
not a runtime surprise.

`toolu-orm-core` and `toolu-orm-cli` default to `libsql`, which is why they are
pulled in with `default-features = false` above when you want a different driver.
The facade and the other crates have no default driver.

## One driver at a time

`toolu-orm-query` compiles its executor, transaction and fetch code only when
**exactly one** driver feature is active. With two drivers on, the builders and
their `to_sql_for` rendering still work, but `.execute()`, `fetch_all` and
friends are not compiled. Applications pick one driver; only the ORM's own test
matrix runs several.

## `#[derive(FromRow)]` and the driver set

The `FromRow` trait changes shape with the active driver set:

| Drivers on `toolu-orm-core` | Method the trait asks for |
|---|---|
| exactly one | `from_row(&Row)` |
| two or more | `from_pg_row`, `from_libsql_row`, `from_rusqlite_row` |

`#[derive(FromRow)]` always emits the `postgres` + `libsql` shape, so it compiles
only where both features are unified. On a single-driver setup the trait asks for
`from_row` and the derive does not provide it — write the impl by hand, see
[Row mapping](../schema/row-mapping.md).

## Toolchain

Rust 1.94, pinned in `rust-toolchain.toml`. Nothing else to install: migrations
are generated from your own binary, so there is no separate CLI to keep in sync
with the library.
