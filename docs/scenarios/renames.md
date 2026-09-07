# Renames

**Feature:** `diff_with_resolver(old_snapshot, new_registry, &resolver)` asks a `RenameResolver` which added/removed pairs are renames. Matched pairs become `Operation::RenameTable` / `Operation::RenameColumn` instead of drop + create, and `generate_sql_for` renders `ALTER TABLE ... RENAME TO` / `RENAME COLUMN ... TO` on both dialects. The default `NoRenames` never matches.
**Drivers:** both dialects (SQL generation).
**Spec:** AC-15.

## What is proven

| Input | Operations | SQL (both dialects) |
|---|---|---|
| old `users`, new `people`, resolver returns `[("users", "people")]` | `RenameTable`, no `CreateTable` / `DropTable` | `ALTER TABLE "users" RENAME TO "people";` |
| column `name` becomes `full_name`, resolver returns `[("name", "full_name")]` | `RenameColumn`, no `AddColumn` / `DropColumn` | `ALTER TABLE "users" RENAME COLUMN "name" TO "full_name";` |
| resolver returns nothing (`NoRenames` and an empty custom resolver) | drop + create / drop + add | no `RENAME` |

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(rename_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | rename_test | no_renames_returns_empty_table_renames |
| default | rename_test | no_renames_returns_empty_column_renames |
| default | rename_test | no_renames_with_empty_inputs |
| default | rename_test | table_rename_produces_rename_table_op_and_no_create_drop_table |
| default | rename_test | table_rename_sql_contains_alter_table_rename_to |
| default | rename_test | column_rename_produces_rename_column_op_and_no_add_drop_column |
| default | rename_test | column_rename_sql_contains_rename_column_to |
| default | rename_test | empty_resolver_yields_drop_and_create_instead_of_rename |
