# Facade crate

**Feature:** `toolu-orm` is a facade over the four library crates. One version and one feature list (`libsql` / `rusqlite` / `postgres`) drive the whole stack, and `toolu_orm::prelude::*` makes the proc macros expand correctly for a consumer whose only dependency is `toolu-orm`.
**Drivers:** driver-agnostic — schema construction and SQL generation, no database.
**Spec:** single-crate install.

## What is proven

| Concern | Proof |
|---|---|
| Re-exports | `toolu_orm::core`, `toolu_orm::query`, `toolu_orm::connection` reach the four crates; the macros are re-exported at the root and in the prelude. |
| Macro expansion | `#[table]` expands with only `toolu-orm` in `[dependencies]` — the expansion names `toolu_orm_core` and `toolu_orm_query` directly, and the prelude glob is what puts those crate names in scope. |
| Generated schema | `table_def()` carries the table name and column order; the companion module exposes `TABLE` and `ALL_COLUMNS`. |
| Generated builders | `select().columns_raw(..).to_sql()` renders through `toolu-orm-query`, proving the facade's feature forwarding reaches it. |

`crates/orm/tests/facade_test.rs` names no `toolu_orm_*` crate: everything it touches has to arrive through the facade, so the test fails if a re-export or a prelude entry is dropped.

Without the prelude the expansion fails to resolve `toolu_orm_core`, because Cargo only puts direct dependencies in a crate's extern prelude. Making the macros emit facade-relative paths (the `proc-macro-crate` approach) would remove the glob requirement and is tracked as a follow-up.

## How to run

```sh
cargo nextest run -p toolu-orm
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | facade_test | table_macro_expands_through_the_facade |
| default | facade_test | generated_builders_reach_the_query_crate |
