# Shared write execution

`InsertBuilder`, `UpdateBuilder`, and `DeleteBuilder` expose async
`execute_on(&impl DbConnection) -> Result<u64, DbError>` across feature combinations.
The connection selects SQL dialect and parameter conversion. Existing `execute`
methods keep their single-driver contracts. Custom connections should override
`DbConnection::dialect`; its compatibility default is `Dialect::CURRENT`.

The same test function writes integer IDs and text containing quotes, placeholder
syntax, SQL punctuation, and Unicode. Subsequent bound filters verify those exact
stored values while UPDATE/DELETE counts prove two matches and zero matches.
Missing-table failures from all three builders remain `DbError::Query`, and a
subsequent valid write succeeds. PostgreSQL uses an isolated schema, rusqlite and
libsql use real in-memory databases (the sqlite-vec lane also checks the loaded extension version), and Lance uses a temporary attached directory
with the pinned extension. Missing services or artifacts fail the tests.

This entry returns affected counts, not RETURNING rows. It adds no conflict,
transaction, or unsupported-capability emulation. [Capability guards](backend-capabilities.md) reject unsupported structured Lance operations before execution.

## Plain Lance INSERT

With the `lancedb` feature and a pinned extension, ordinary `InsertBuilder`
VALUES calls bind integer, real, text, boolean, blob and NULL values. Each
builder writes one row; repeated calls are separate statements, not an atomic
batch. Exact values survive a fresh connection, including empty text/blob,
negative integers, false, and SQL-looking Unicode text.

`select` / `select_raw` sends one INSERT SELECT to the database without loading
source rows into Rust. The real-data test copies two filtered rows, supplies a
bound projected literal, checks the exact reopened target, and proves an empty
source returns zero without changing the target or source. Missing tables,
incompatible projection width and deferred codecs propagate query errors; the
existing rows survive and a later valid insert succeeds. Conflict modes and
RETURNING still refuse, including INSERT SELECT RETURNING with no source matches.

Run `bash scripts/check-lancedb-smoke.sh`; it checks this inventory and executes
the tests with the checksum-pinned real extension. These are INSERT guarantees,
not validation of every possible SELECT expression or an atomic multi-call API.

## Plain Lance UPDATE

`UpdateBuilder::execute_on` executes bound `set`, computed `set_scalar`, and
trusted raw `set_expr` assignments on attached Lance tables. Two-row fixtures
prove selective updates survive reopening: exact text (including SQL-looking
Unicode), real, boolean and NULL assignments affect only the selected ID. Computed assignments bind before typed WHERE predicates; distinct bound
values and exact reopened rows prove their order on the real backend.

The returned count is the number of matched rows, including a row assigned its
existing value. No match returns zero and preserves reopened data. An unfiltered
UPDATE affects every row. Missing tables/columns, an empty assignment list and
deferred codecs propagate query errors without changing rows; a subsequent valid
update succeeds and persists. Raw expression text is caller-owned SQL, not a
portable translation. This slice adds no DELETE, MERGE or RETURNING guarantees.

Lance UPDATE rejects bound BLOB literals, both empty and nonempty, with a
`DbError::Query` naming the unsupported BLOB literal. A mixed text/BLOB assignment
fails without a partial write; INSERT blob support does not imply UPDATE support.

The same pinned `bash scripts/check-lancedb-smoke.sh` command checks and executes
the dedicated `lance_update_test` target.

## Tests

| Lane | Binary | Test |
| --- | --- | --- |
| postgres | portable_write_test | postgres::bound_writes_counts_and_errors |
| rusqlite-only | portable_write_test | sqlite::bound_writes_counts_and_errors |
| libsql-only | portable_write_test | libsql::bound_writes_counts_and_errors |
| lancedb-smoke | portable_write_test | lance::bound_writes_counts_and_errors |
| lancedb-smoke | portable_write_test | lance_capabilities::unsupported_mutations_preserve_rows_after_reopen |
| lancedb-smoke | portable_write_test | lance_capabilities::transaction_and_constraint_requirements_refuse_before_writes |
| lancedb-smoke | portable_write_test | lance_insert::bound_scalar_rows_persist_after_reopen |
| lancedb-smoke | portable_write_test | lance_insert::insert_select_copies_projection_and_empty_source_persists |
| lancedb-smoke | portable_write_test | lance_insert::invalid_insert_preserves_rows_and_session_recovers |
| lancedb-smoke | lance_update_test | selective_scalar_assignments_persist_after_reopen |
| lancedb-smoke | lance_update_test | computed_and_raw_assignments_bind_before_typed_filters |
| lancedb-smoke | lance_update_test | no_match_same_value_and_unfiltered_counts_persist |
| lancedb-smoke | lance_update_test | invalid_updates_preserve_rows_and_session_recovers |
| default | facade_only_write_test | shared_writes_compile_with_facade_only |

The facade proof compiles with only `toolu-orm` as a direct dependency. The Lance
smoke script also compiles mixed `postgres,rusqlite,lancedb` query features.

