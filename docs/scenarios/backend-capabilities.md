# Backend capability refusal

Shared `InsertBuilder`, `UpdateBuilder`, and `DeleteBuilder::execute_on` check the
connection's runtime dialect before rendering or sending a statement. Lance
conflict policies (explicit `OnConflict`, `or_ignore`, `or_replace`, including
INSERT SELECT) and ordinary DML `RETURNING` return
`DbError::UnsupportedCapability { backend, capability }`. The checks inspect
structured builder state, never SQL strings. An insert requesting both reports
`OnConflict` first. Zero-match UPDATE/DELETE still refuse `RETURNING`.

`validate_for(Dialect)` exposes the same preflight when callers render manually.
For those three builders, `to_sql_for` itself remains a renderer. MergeBuilder
validates in `to_sql_for` as well. Bound SQL-looking text and conflict-column
hints without an active conflict policy do not trigger refusal. Raw SQL and raw
fragments remain caller-owned escape hatches; these checks do not parse them.

The facade exports the API through `toolu_orm::connection`. Each diagnostic
contains the backend, stable capability name below, and an explicit alternative.

| Capability | Stable name | Alternative and limit |
| --- | --- | --- |
| `KeyMerge` | `key_merge` | Native MERGE on Lance/PostgreSQL 15+; SQLite ON CONFLICT remains a separate unique-key API. |
| `OnConflict` | `on_conflict` | PostgreSQL/SQLite for unique-key conflict handling; explicit MergeBuilder has different semantics and does not enforce uniqueness. |
| `DmlReturning` | `dml_returning` | Write without RETURNING then SELECT explicitly; the separate read is not atomic with the write. |
| `PrimaryKey` | `primary_key` | PostgreSQL/SQLite for enforced uniqueness. |
| `UniqueConstraint` | `unique_constraint` | PostgreSQL/SQLite for enforced uniqueness. |
| `UniqueIndex` | `unique_index` | PostgreSQL/SQLite; a Lance scalar index does not enforce uniqueness. |
| `ForeignKey` | `foreign_key` | PostgreSQL or SQLite with foreign-key enforcement enabled. |
| `NotNull` | `not_null` | PostgreSQL/SQLite for database enforcement; application validation is not equivalent. |
| `CheckConstraint` | `check_constraint` | PostgreSQL/SQLite for database enforcement; application validation is not equivalent. |
| `MultiStatementTransaction` | `multi_statement_transaction` | PostgreSQL/SQLite transactions for multi-statement atomicity. |

`DbConnection::require_capabilities(&[Capability])` and the free
`require_capabilities(Dialect, &[Capability])` support explicit preflight. Lance
permits KeyMerge but refuses the other requirements; an empty list passes.
SQLite refuses KeyMerge; PostgreSQL passes this coarse backend check. It does not enable connection
settings or validate arbitrary statements. Constraint-bearing schema operations
must call preflight before mutation. The current `LanceColumn` lifecycle API
has no constraint declarations; automatic schema/migration integration is a
separate feature.

`LanceDbConnection::begin().await` always returns the named transaction error
before SQL or locks. Its success type is `Infallible`: there is no portable Lance
transaction handle. Narrow DML rollback observations do not establish general
transaction support, particularly with DDL. Raw batches do not acquire this
portability guarantee.


Native key-based MERGE uses the separate `KeyMerge` / `key_merge` capability.
Lance and PostgreSQL 15+ support it; SQLite refuses it before execution. See
[portable writes](portable-writes.md#explicit-key-based-merge) for policies,
source-key validation and the absence of a uniqueness guarantee.

## Real-data evidence

The pinned Lance lane seeds a real temporary dataset and asserts identical rows
after every refusal, then closes and reopens the session. It tests conflict modes,
INSERT SELECT, missing-table precedence, all three DML RETURNING forms and
zero-match mutations. The same empty guard permits a builder-rendered filtered
SELECT with SQL-looking text. The transaction/constraint case checks BEGIN refusal,
checks every constraint before a guarded CREATE, verifies no extra table exists
after reopen, and executes a supported write afterward. Test names are registered
in [shared writes](portable-writes.md) and verified by the smoke runner.

Run `bash scripts/check-lancedb-smoke.sh` with the pinned real extension; the
ordinary driver lanes additionally run existing shared write regression tests.

## Tests

| Lane | Binary | Test |
| --- | --- | --- |
| default | capability_test | requirements_are_named_and_runtime_selected |
| default | capability_test | conflict_modes_and_returning_use_structured_state |
| default | capability_test | names_values_and_inactive_conflict_hints_do_not_trigger_refusal |
