# Postgres row-level security

**Feature:** a table opts into Postgres row-level security in the schema — `#[policy("name", for = select, as = restrictive, to = ["role"], using = "…", with_check = "…")]` on a `#[table]` struct, or `#[table(rls = "enable" | "force")]` for a policy-less default-deny table — and `generate` diffs the declaration into `ALTER TABLE … ENABLE | DISABLE ROW LEVEL SECURITY`, `… FORCE | NO FORCE ROW LEVEL SECURITY`, `CREATE POLICY` and `DROP POLICY IF EXISTS`. `TableDef.row_security: Option<RowSecurity>` carries it; any declared policy implies `ENABLE`, as in Drizzle. A changed policy is dropped and created again (`ALTER POLICY` cannot change `FOR` or `AS`). `diff` refuses, before writing anything, a policy with neither expression, `USING` on `FOR INSERT`, `WITH CHECK` on `FOR SELECT` / `DELETE`, a duplicate name, and any policy on a virtual table. At runtime, `PgTransaction::set_local_config(name, value)` binds both arguments through `set_config($1, $2, true)` so the per-request context a policy reads — `current_setting('app.tenant_id', true)` — is never formatted into SQL text.
**Drivers:** Postgres executes all of it. libsql and rusqlite have no row security: every operation renders as a `--` comment, the same treatment enums and foreign-key constraints already get, and the migration still applies. Roles are not managed; `TO` names whatever roles exist.
**Not covered:** `ALTER POLICY`; policy expressions are opaque strings, so a column they name can be dropped without the diff noticing (Postgres then rejects the `DROP COLUMN`).

## What is proven

- Types: the `PolicyDef` builder, the SQL keywords, which clauses each command accepts, and the JSON shape — defaults omitted, so a snapshot written before this feature is byte-identical.
- Diff: enable-then-create for a new table, add / change / drop one policy at a time, a `FORCE` flip alone, disable with drops when the declaration goes, policies surviving a `RenameResolver` rename, and the six refusals (including row security on a virtual table, with or without a policy).
- SQL: each operation on Postgres (clause order `AS … FOR … TO … USING … WITH CHECK`, keyword roles `PUBLIC` / `CURRENT_USER` unquoted, others quoted with an embedded `"` doubled) and as comments on SQLite; ordering puts `DropPolicy` with the drops and the flags before the `CreatePolicy` they govern.
- Macro: `#[policy]` and `rls = …` land on `TableDef.row_security`; the five malformed forms are pinned under [Macro compile errors](macro-compile-errors.md).
- Live Postgres: generate → migrate → evolve → generate → migrate over three versions, read back from `pg_class.relrowsecurity` / `relforcerowsecurity` and `pg_policies` (kind, command, roles, `qual`, `with_check`); then, for a `NOLOGIN` role the test creates and switches to with `SET LOCAL ROLE` (the test user is a superuser and the owner, both exempt), tenant 1 sees two rows and inserts a third, tenant 2 sees one and gets SQLSTATE `42501` for a cross-tenant insert, and no tenant at all sees nothing.
- `set_local_config`: visible for the transaction and reset after it, a hostile value stays a value, a one-part name is rejected with `42704`.

The tenant expression the live fixture uses is `tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::int`. `current_setting(…, true)` is `NULL` before a session ever set the name but the empty string after a `SET LOCAL` was reset, and `''::int` is an error; the `NULLIF` makes an unset context match no row either way.

## How to run

```sh
cargo nextest run -p toolu-orm-core -E 'binary(/policy_.*_test/)'
cargo nextest run -p toolu-orm-macros -E 'binary(policy_macro_test) | binary(compile_fail_test)'
cargo nextest run -p toolu-orm-cli -E 'binary(rls_sqlite_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-cli -p toolu-orm-connection --features postgres \
  -E 'binary(rls_postgres_test) | binary(postgres_live_session_config_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | policy_test | builder_sets_every_clause |
| default | policy_test | defaults_are_permissive_for_all_to_public |
| default | policy_test | command_keywords_and_clause_rules |
| default | policy_test | row_security_constructors |
| default | policy_test | minimal_policy_serializes_to_name_and_expression_only |
| default | policy_test | full_policy_round_trips_through_json |
| default | policy_test | row_security_json_omits_defaults |
| default | policy_diff_test | changes::new_table_with_policy_enables_then_creates |
| default | policy_diff_test | changes::unchanged_declaration_diffs_to_nothing |
| default | policy_diff_test | changes::adding_a_policy_creates_only_that_policy |
| default | policy_diff_test | changes::changed_expression_drops_and_recreates |
| default | policy_diff_test | changes::flipping_force_alters_flags_only |
| default | policy_diff_test | changes::removing_the_declaration_drops_policies_and_disables |
| default | policy_diff_test | changes::renamed_table_keeps_its_policies |
| default | policy_diff_test | refusals::refuses_a_policy_without_any_expression |
| default | policy_diff_test | refusals::refuses_using_on_insert_and_with_check_on_select |
| default | policy_diff_test | refusals::refuses_duplicate_policy_names |
| default | policy_diff_test | refusals::refuses_row_security_on_a_virtual_table |
| default | policy_sql_test | enable_renders_both_flags_postgres |
| default | policy_sql_test | disable_renders_disable_and_no_force_postgres |
| default | policy_sql_test | minimal_policy_renders_name_table_and_using |
| default | policy_sql_test | full_policy_renders_every_clause_in_postgres_order |
| default | policy_sql_test | drop_policy_postgres |
| default | policy_sql_test | every_row_security_operation_is_a_comment_on_sqlite |
| default | policy_sql_test | role_with_embedded_quote_is_doubled |
| default | policy_ordering_test | drop_policy_before_add_column_and_creates_after |
| default | policy_ordering_test | diff_order_keeps_flags_before_policies_within_the_create_tier |
| default | policy_snapshot_test | undeclared_row_security_is_absent_from_the_json |
| default | policy_snapshot_test | declared_row_security_round_trips |
| default | policy_snapshot_test | snapshot_without_the_field_reads_as_undeclared |
| default | policy_macro_test | policies_imply_enabled_row_security |
| default | policy_macro_test | rls_force_without_policies_is_forced_default_deny |
| default | policy_macro_test | table_without_the_attributes_declares_none |
| default | rls_sqlite_test | row_security_renders_as_comments_and_migrates |
| postgres | rls_postgres_test | migration::loop_applies_enable_policies_force_and_disable |
| postgres | rls_postgres_test | migration::unchanged_declaration_generates_nothing |
| postgres | rls_postgres_test | enforcement::policy_filters_reads_and_writes_per_tenant |
| postgres | postgres_live_session_config_test | setting_is_visible_inside_and_reset_after_the_transaction |
| postgres | postgres_live_session_config_test | value_is_bound_not_interpolated |
| postgres | postgres_live_session_config_test | rejected_name_is_a_query_error |
