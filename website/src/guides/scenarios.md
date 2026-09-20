# Scenarios

Every feature has a page in
[`docs/scenarios/`](https://github.com/Falconiere/toolu-orm/tree/main/docs/scenarios)
naming the tests that prove it on each supported driver. The pages record shared
behavior, dialect-specific features and intentional differences, such as
SQLite replacement versus Postgres conflict updates.

`scripts/check-scenario-docs.sh` runs in CI. It fails when a test listed on a
page does not exist, or when a test in a documented binary appears on no page —
so adding or renaming a test means updating its page in the same change, and the
pages cannot quietly go stale.

Selected scenarios (the [full index](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/README.md) also covers FTS5, vectors, schema changes and maintenance):

| Scenario | What it proves |
|---|---|
| [Upsert](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/upsert.md) | Explicit `on_conflict`, `RETURNING`, and the dialect-specific `or_replace` / `or_ignore` shorthands. |
| [Relational loads](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/relational-loads.md) | `with_many` / `with_one` in one statement, including empty and null relations. |
| [Value round-trip](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/value-round-trip.md) | Every `Value` variant binds and reads back unchanged. |
| [Fetch semantics](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/fetch-semantics.md) | `fetch_one` / `fetch_optional` / `count` / `exists` on empty, single and multi-row sets. |
| [Filters](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/filters.md) | Every operator against real rows, parameter numbering, nested AND/OR, empty `in_list`. |
| [Transactions](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/transactions.md) | Commit persists; rollback and drop discard; reads see own writes. |
| [Postgres connection](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/postgres-connection.md) | Pool checkout timeouts, prepared-statement caching and eviction, connection and SQLSTATE error mapping. |
| [FromRow derive](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/from-row-derive.md) | The derive on each driver shape against real rows, `NULL` to `None`, missing columns, `with` conversions. |
| [Migration loop](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/migration-loop.md) | generate → migrate → evolve → generate → migrate, asserted through `PRAGMA` / `information_schema`. |
| [Migration failures](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/migration-failures.md) | Rollback after a failing statement, malformed journal, absent directory. |
| [Expression fragments](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/expr-fragments.md) | `Expr` fragments with parameter offsets per dialect, nesting, empty-list constant. |
| [Legacy snapshot](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/legacy-snapshot.md) | Old snapshot JSON still deserializes and diffs. |
| [Renames](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/renames.md) | A `RenameResolver` turns drop+create into `RENAME`. |
| [Macro compile errors](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/macro-compile-errors.md) | Each proc-macro error message pinned by trybuild. |
| [Facade crate](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/facade.md) | `#[table]` and the builders work with `toolu-orm` as the only dependency. |
| [Query composition](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/query-composition.md) | CTEs, UNION, subqueries and table-valued sources. |
| [Scalar expressions](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/scalar-expressions.md) | Bound function arguments, arithmetic, CASE and LIKE ESCAPE. |
| [Reusable bound parameters](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/reusable-bound-parameters.md) | Shared handles reuse placeholders across predicates and nested statements. |
| [INSERT … SELECT](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/insert-select.md) | Copy rows within the database, including attached SQLite databases. |
| [Blocking connection](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/blocking-connection.md) | rusqlite query and migration APIs without a runtime. |
| [Migration history integrity](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/migration-history-integrity.md) | Revalidate applied hashes before processing pending migrations. |
| [Embedded migrations](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/embedded-migrations.md) | Apply, baseline and inspect SQL compiled into the binary. |
| [Lanes](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/lanes.md) | Which CI lane compiles which suite. |

If you are evaluating the ORM, these pages are the honest answer to "does it
actually do that on my database" — each row points at a test you can run.
