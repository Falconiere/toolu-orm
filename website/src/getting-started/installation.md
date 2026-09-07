# Installation

The five crates are published to crates.io and share one workspace version.

```toml
[dependencies]
toolu-orm-core       = { version = "0.1", default-features = false, features = ["libsql"] }
toolu-orm-macros     = { version = "0.1", features = ["libsql"] }
toolu-orm-query      = { version = "0.1", features = ["libsql"] }
toolu-orm-connection = { version = "0.1", features = ["libsql"] }
toolu-orm-cli        = { version = "0.1", default-features = false, features = ["libsql"] }
tokio                = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Only `toolu-orm-core` and `toolu-orm-macros` are mandatory. Add `toolu-orm-query`
for builders, `toolu-orm-connection` for a connection, and `toolu-orm-cli` for
migrations.

## Driver features

Every crate exposes the same three features — `libsql`, `rusqlite`, `postgres` —
and forwards them to `toolu-orm-core`.

| Feature | Backing crate | Mode |
|---|---|---|
| `libsql` | [libsql](https://crates.io/crates/libsql) | async; local file, `:memory:`, or a Turso embedded replica |
| `rusqlite` | [rusqlite](https://crates.io/crates/rusqlite) (bundled) | sync, wrapped in `spawn_blocking` |
| `postgres` | [tokio-postgres](https://crates.io/crates/tokio-postgres) + [deadpool-postgres](https://crates.io/crates/deadpool-postgres) | async pool, rustls TLS |

**Enable the drivers you need on every crate you depend on.** Cargo unifies
features across the dependency graph, and the generated code changes shape with
the active set — a driver enabled on one crate but not another produces a
mismatch at compile time, not at runtime.

`toolu-orm-core` and `toolu-orm-cli` default to `libsql`, which is why they are
pulled in with `default-features = false` above when you want a different driver.
The other three crates have no default driver.

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

`#[derive(FromRow)]` currently emits the `postgres` + `libsql` shape (its libsql
method is an error stub), so it compiles when both features are on. With a single
driver, write the impl by hand — it is a few lines, see
[Row mapping](../schema/row-mapping.md). Making the derive follow the active
driver set is tracked as a follow-up.

## Toolchain

Rust 1.94 (edition 2024), pinned in `rust-toolchain.toml`. Nothing else to
install: migrations are generated from your own binary, so there is no separate
CLI to keep in sync with the library.
