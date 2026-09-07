# Transactions

**Feature:** three transaction surfaces, one contract: commit persists, anything else discards.
- libsql (orm-query): `conn.run_transaction(|tx| async move { ... })` commits on `Ok`, rolls back on `Err` (`TransactionExt`).
- Postgres (orm-query): `PgTransaction::new(client.transaction().await?)` implements `Executor`; `commit()`, `rollback()`, or drop.
- Postgres (orm-connection): `PgConnection::transaction()` returns a `PgTransaction` implementing `DbConnection`; `commit()` or drop.
- rusqlite: no transaction API yet (spec Non-goal 3).

**Drivers:** libsql, Postgres.
**Spec:** AC-3 (Postgres), revived libsql suites (AC-1).

## What is proven

| Assertion | libsql | Postgres (orm-query `PgTransaction`) | Postgres (orm-connection) |
|---|---|---|---|
| commit persists the insert | `transaction_commit_on_ok`, `transaction_commit` | `commit_persists_the_write` | `commit_persists_the_write` |
| `Err` / `rollback()` discards | `transaction_rollback_on_err`, `transaction_rollback` | `rollback_discards_the_write` | (drop) |
| drop without commit discards | (closure returns `Err`) | `dropping_without_commit_discards_the_write` | `dropping_without_commit_rolls_back` |
| reads inside see own writes | `transaction_multiple_operations` | `reads_inside_the_transaction_see_its_own_writes` | `reads_inside_the_transaction_see_its_own_writes` |
| a failing statement inside, then drop | — | — | `failed_statement_inside_transaction_rolls_back_on_drop` |

Every case reads the table back through a separate query after the transaction ends, on the same live connection.

## How to run

```sh
cargo nextest run -p toolu-orm-query --features libsql -E 'binary(transaction_test) | test(/transaction_/)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(postgres_transaction_test) | binary(postgres_live_transaction_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| libsql-only | transaction_test | transaction_commit_on_ok |
| libsql-only | transaction_test | transaction_rollback_on_err |
| libsql-only | transaction_test | transaction_multiple_operations |
| libsql-only | integration_test | mutation_queries::transaction_commit |
| libsql-only | integration_test | mutation_queries::transaction_rollback |
| postgres | postgres_transaction_test | commit_persists_the_write |
| postgres | postgres_transaction_test | rollback_discards_the_write |
| postgres | postgres_transaction_test | dropping_without_commit_discards_the_write |
| postgres | postgres_transaction_test | reads_inside_the_transaction_see_its_own_writes |
| postgres | postgres_live_transaction_test | commit_persists_the_write |
| postgres | postgres_live_transaction_test | dropping_without_commit_rolls_back |
| postgres | postgres_live_transaction_test | reads_inside_the_transaction_see_its_own_writes |
| postgres | postgres_live_transaction_test | failed_statement_inside_transaction_rolls_back_on_drop |
