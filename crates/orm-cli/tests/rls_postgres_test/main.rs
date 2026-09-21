//! Row-level security against a live Postgres: the migration loop applies
//! `ENABLE` / `FORCE ROW LEVEL SECURITY` and `CREATE` / `DROP POLICY` through
//! generate → migrate, asserted through `pg_class` and `pg_policies`; and the
//! applied policies actually filter rows for a non-owner role whose tenant
//! context was set with `PgTransaction::set_local_config`.
//!
//! Needs `docker compose -f docker-compose.test.yaml up -d --wait` and
//! `TEST_DB_PORT=5434` locally. Runs on the five-crate postgres lane only.
#![cfg(all(feature = "libsql", feature = "postgres"))]

#[path = "../fixtures/rls_registry.rs"]
pub mod rls_registry;

mod enforcement;
mod migration;
mod support;
