# Facade crate

**Feature:** `toolu-orm` is a facade over the four library crates. One version and one feature list (`libsql` / `rusqlite` / `postgres`) drive the whole stack, and every proc macro expands correctly for a consumer whose only dependency is `toolu-orm`.
**Drivers:** driver-agnostic — schema construction and SQL generation, no database. The `FromRow` derive case runs on every lane, including the default one where `toolu-orm-core` has libsql alone.
**Spec:** single-crate install.

## What is proven

| Concern | Proof |
|---|---|
| Re-exports | `toolu_orm::core`, `toolu_orm::query`, `toolu_orm::connection` reach the four crates; the macros are re-exported at the root and in the prelude. |
| Macro paths | Every expansion emits absolute paths resolved from the consumer's `Cargo.toml` (`crates/orm-macros/src/paths.rs`): `::toolu_orm::core::…` behind the facade, `::toolu_orm_core::…` for a direct dependent. Nothing has to be in scope. |
| Companion column module | `#[table]`'s nested `mod` holds `Column<T>` constants that resolve without a prelude — the case a `use` in the parent module could never reach. |
| Generated schema | `table_def()` carries the table name and column order; the companion module exposes `TABLE` and `ALL_COLUMNS`. |
| Generated builders | `select()` renders through `toolu-orm-query`, with a typed column driving `ORDER BY`, proving the facade's feature forwarding reaches it. |
| Views and derives | `#[view]`, `#[derive(ColumnEnum)]`, `#[derive(Relational)]` and `#[derive(FromRow)]` all expand under the single-dependency constraint, reaching `serde` / `serde_json` / the driver row types through `toolu-orm-core`'s re-exports. `FromRow` additionally resolves `impl_derived_from_row!` — a `macro_rules!` at `toolu-orm-core`'s root — through the facade re-export. |
| `missing_docs` | `#[table]`, `#[fts5_table]` and `#[vec0_table]` emit `#[doc = "…"]` on the companion module, its constants, and the builder methods, so a `pub mod` of macro tables compiles under `#![deny(missing_docs)]`. |
| Prelude | `toolu_orm::prelude` still exports the macros and the crate names, for code that writes `toolu_orm_core::…` itself. |

The single-dependency constraint lives in `crates/orm-facade-consumer/Cargo.toml`,
whose `[dependencies]` hold `toolu-orm` and nothing else. Cargo fills a crate's
extern prelude from direct dependencies only, so that package reproduces an
external consumer exactly. Adding a second dependency there would silently
delete the proof.

`crates/orm/tests/facade_test.rs` cannot stand in for it: the `toolu-orm`
package depends on the four library crates directly, so `toolu_orm_core` is in
its extern prelude whatever the facade re-exports. It covers the re-exports and
the prelude instead.

## How to run

```sh
cargo nextest run -p toolu-orm -p toolu-orm-facade-consumer
```

That includes `facade_only_from_row_test`: the `FromRow` derive follows
whichever shape `toolu-orm-core` compiled, so it needs no particular driver and
the default lane exercises the single-driver shape. It runs on the postgres lane
too, where the two-driver shape applies:

```sh
cargo nextest run -p toolu-orm-facade-consumer --features postgres
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | facade_test | table_macro_expands_through_the_facade |
| default | facade_test | generated_builders_reach_the_query_crate |
| default | facade_test | prelude_still_exports_the_expansion_crate_names |
| default | facade_only_test | table_macro_expands_without_the_prelude |
| default | facade_only_test | companion_column_module_holds_typed_columns |
| default | facade_only_test | generated_builders_reach_the_query_crate |
| default | facade_only_test | fts5_table_macro_expands_without_the_prelude |
| default | facade_only_test | vec0_table_macro_expands_without_the_prelude |
| default | facade_only_test | view_struct_serializes_through_the_re_exported_serde |
| default | facade_only_test | column_enum_derive_reports_renamed_variants |
| default | facade_only_test | relational_derive_decodes_a_json_row |
| default | facade_only_test | relational_derive_decodes_a_null_relation_as_empty |
| default | facade_only_from_row_test | from_row_derive_reports_required_columns |
| default | facade_only_missing_docs_test | missing_docs_pub_mod_compiles_with_macro_tables |
