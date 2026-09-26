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
transaction, or unsupported-capability emulation; #174 owns capability guards.

## Tests

| Lane | Binary | Test |
| --- | --- | --- |
| postgres | portable_write_test | postgres::bound_writes_counts_and_errors |
| rusqlite-only | portable_write_test | sqlite::bound_writes_counts_and_errors |
| libsql-only | portable_write_test | libsql::bound_writes_counts_and_errors |
| lancedb-smoke | portable_write_test | lance::bound_writes_counts_and_errors |
| default | facade_only_write_test | shared_writes_compile_with_facade_only |

The facade proof compiles with only `toolu-orm` as a direct dependency. The Lance
smoke script also compiles mixed `postgres,rusqlite,lancedb` query features.

