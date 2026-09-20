# toolu-orm-macros

Proc macros for toolu-orm — `#[table]`, `#[fts5_table]`, `#[vec0_table]`, and the `FromRow`, `ColumnEnum`, `Relational` derives.

## Crate Type
- Proc-macro library
- Internal deps: toolu-orm-core

## Crate-Specific Rules
- This is a proc-macro crate — can only export procedural macros
- `#[table]` generates: cleaned struct, TableSchema impl, builder methods, Column\<T\> constants module, view structs
- `#[derive(FromRow)]` emits one decoder per driver plus a call to `toolu_orm_core::impl_derived_from_row!`, which keeps the ones orm-core compiled a method for — the derive never reads its own driver features
- `#[derive(FromRow)]` supports `#[from_row(with = "fn_name")]` on every driver; the function takes and returns the field's own type
- `#[derive(ColumnEnum)]` generates `EnumSchema::variants()` for CHECK constraints
- Index attributes: `#[index("name", col, desc(other), where = "predicate")]` and `#[unique_index("name", col)]`; these are parsed by `#[table]`
- View attributes: `#[view(Name, pick(a, b))]` / `#[view(Name, omit(c))]` generate subset structs; they are parsed by `#[table]`, not standalone proc macros

## Key Modules
- `lib.rs` — Entry point: three attribute macros and three derives
- `parse/column_parsing.rs` — Walk struct fields into column definitions
- `parse/column_attrs.rs` — Read the `#[column(...)]` keys, and strip the attribute before re-emission
- `parse/column_type_spec.rs` — The field's Rust type as a `TypeSpec` (`Varchar<N>`, `Char<N>`)
- `parse/index_parsing.rs` — Parse `#[index]`/`#[unique_index]` attributes
- `expand/schema_expansion.rs` — Generate TableSchema impl + constants
- `expand/columns_expansion.rs` — Generate Column\<T\> constants module
- `column_enum.rs` — ColumnEnum derive logic
- `from_row.rs` — FromRow field and `#[from_row(...)]` parsing
- `from_row_expand.rs` — per-driver row reads and the `impl_derived_from_row!` call

## References
- Root CLAUDE.md (project-wide rules)
- Root `Cargo.toml`, `clippy.toml` and `scripts/check-file-length.sh`
