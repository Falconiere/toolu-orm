# toolu-orm-macros

Proc macros for toolu-orm — `#[table]`, `#[derive(FromRow)]`, `#[derive(ColumnEnum)]`.

## Crate Type
- Proc-macro library
- Internal deps: toolu-orm-core

## Crate-Specific Rules
- This is a proc-macro crate — can only export procedural macros
- `#[table]` generates: cleaned struct, TableSchema impl, builder methods, Column\<T\> constants module, view structs
- `#[derive(FromRow)]` supports `#[from_row(with = "fn_name")]` for custom field conversions
- `#[derive(ColumnEnum)]` generates `EnumSchema::variants()` for CHECK constraints
- Index attributes: `#[index(name, columns, unique)]` and `#[unique_index(name, columns)]`
- View attributes: `#[view(name, columns)]` generates subset struct

## Key Modules
- `lib.rs` — Entry point: 3 proc macros
- `parse/column_parsing.rs` — Parse `#[column(...)]` attributes
- `parse/index_parsing.rs` — Parse `#[index]`/`#[unique_index]` attributes
- `expand/schema_expansion.rs` — Generate TableSchema impl + constants
- `expand/columns_expansion.rs` — Generate Column\<T\> constants module
- `column_enum.rs` — ColumnEnum derive logic
- `from_row.rs` — FromRow derive logic

## References
- Root CLAUDE.md (project-wide rules)
- `docs/rules/forbidden-syntax-rust.md`
