# Row-level security

Postgres can filter every statement on a table through policies the database
enforces itself, so a query that forgets its `WHERE tenant_id = …` still
cannot read or write another tenant's rows. toolu-orm declares the policies in
the schema and migrates them like any other table change.

```rust
use toolu_orm_core::column::{BigInt, Boolean, Text};
use toolu_orm_macros::table;

#[table(name = "docs", rls = "force")]
#[policy(
  "tenant_isolation",
  using = "tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::int"
)]
#[policy("hide_archived", for = select, as = restrictive, using = "NOT archived")]
#[policy("writers_insert", for = insert, to = ["app_writer"], with_check = "owner = current_user")]
pub struct Docs {
  #[column(primary_key)]
  pub id: Text,
  #[column(not_null)]
  pub tenant_id: BigInt,
  pub owner: Text,
  #[column(not_null, default = "false")]
  pub archived: Boolean,
}
```

`generate` turns that into:

```sql
ALTER TABLE "docs" ENABLE ROW LEVEL SECURITY;
ALTER TABLE "docs" FORCE ROW LEVEL SECURITY;
CREATE POLICY "tenant_isolation" ON "docs" USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::int);
CREATE POLICY "hide_archived" ON "docs" AS RESTRICTIVE FOR SELECT USING (NOT archived);
CREATE POLICY "writers_insert" ON "docs" FOR INSERT TO "app_writer" WITH CHECK (owner = current_user);
```

## Declaring it

| Attribute | Effect |
|---|---|
| `#[policy("name", …)]` | One `CREATE POLICY`. Repeatable; names are unique per table. Declaring any policy enables row security on the table. |
| `for = all \| select \| insert \| update \| delete` | The command the policy applies to. Defaults to `all`. |
| `as = permissive \| restrictive` | Permissive policies are OR-ed together; restrictive ones are AND-ed on top of that. Defaults to `permissive`. |
| `to = "role"` / `to = ["role", …]` | The roles the policy applies to. Defaults to `PUBLIC`. `public`, `current_role`, `current_user` and `session_user` render as keywords; anything else is quoted. |
| `using = "…"` | Raw SQL boolean over existing rows: which rows are visible, updatable, deletable. |
| `with_check = "…"` | Raw SQL boolean over new rows on `INSERT` / `UPDATE`. A `FOR ALL` or `FOR UPDATE` policy with only `using` applies that expression to new rows too. |
| `#[table(name = "…", rls = "enable")]` | Enable row security without a policy: default-deny for everyone but the owner and superusers. |
| `#[table(name = "…", rls = "force")]` | Also bind the table owner to the policies. |

Without a macro, set `TableDef.row_security` to
`Some(RowSecurity::enabled().policy(PolicyDef::new("tenant_isolation").using("…")))`;
`RowSecurity::forced()` is `FORCE`, and `PolicyDef` has `kind`, `command`,
`role`, `using` and `with_check` builders.

`generate` refuses, before writing a file, what Postgres would refuse: a policy
with neither expression, `using` on `for = insert`, `with_check` on
`for = select` or `for = delete`, two policies with one name, and a policy on a
virtual table.

## What migrates

The diff treats a table's row security as one unit. A new declaration emits the
flags then its policies; a removed one drops every policy then disables the
flags; a changed policy is dropped and created again, because `ALTER POLICY`
cannot change `FOR` or `AS`; a `FORCE` flip alone emits only the two flag
statements. The flags are always rendered as a pair (`ENABLE` / `DISABLE` and
`FORCE` / `NO FORCE`) so a migration states the full target.

SQLite has no row-level security. On `Dialect::Sqlite` every one of these
operations renders as a `-- … (Postgres only; SQLite has no row-level security)`
comment, the migration still applies, and the table has no filtering. Expect
that difference rather than parity here.

Roles are not part of the schema. `to = ["app_writer"]` assumes `app_writer`
exists; create roles and grants outside the migration loop.

## Setting the context

A policy such as `current_setting('app.tenant_id', true)` reads a setting the
application must put in place before the statement runs. `SET LOCAL` cannot
take bind parameters, so hand-writing it means formatting the tenant into SQL
text on the one code path meant to enforce isolation. Use the transaction
helper instead, which binds both arguments through `set_config($1, $2, true)`:

```rust
let mut conn = pg.connect().await?;
let tx = conn.transaction().await?;
tx.set_local_config("app.tenant_id", &tenant_id.to_string()).await?;
Docs::select().columns_raw(&["id"]).fetch_all(&tx).await?;   // only this tenant's rows
tx.commit().await?;                                           // setting is gone
```

The setting lives for that transaction only, so a pooled connection never
carries one request's tenant into the next. Two Postgres details worth
knowing:

- Superusers, roles with `BYPASSRLS`, and (unless `rls = "force"`) the table
  owner bypass every policy. An application that connects as the owner sees
  no filtering at all; connect as, or `SET LOCAL ROLE` to, a role the policies
  apply to.
- `current_setting(name, true)` is `NULL` before the session ever set `name`
  but the empty string after a `SET LOCAL` was reset, and `''::int` is an
  error. Wrap it in `NULLIF(…, '')` so an unset context matches no row.

The full test list, including the live proof that tenant 2 cannot read or
insert tenant 1's rows, is in the
[Postgres row-level security](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/postgres-row-level-security.md)
scenario.
