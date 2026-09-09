mod apply;
mod apply_blocking;
mod baseline;
mod baseline_blocking;
mod ddl;
pub(crate) mod embedded;
mod embedded_blocking;
mod error;
mod missing_extension;
mod pending;
mod run;
mod run_blocking;
mod sql;
mod store;
mod transaction;
mod transaction_blocking;

pub use baseline::{mark_applied, mark_applied_through};
pub use baseline_blocking::{mark_applied_blocking, mark_applied_through_blocking};
pub use ddl::migrations_table_ddl;
pub use embedded::{
  mark_applied_embedded, mark_applied_through_embedded, run_migrate_embedded, EmbeddedMigration,
};
pub use embedded_blocking::{
  mark_applied_embedded_blocking, mark_applied_through_embedded_blocking,
  run_migrate_embedded_blocking,
};
pub use error::MigrateError;
pub use run::run_migrate;
pub use run_blocking::run_migrate_blocking;
pub use store::{
  ensure_migrations_table, ensure_migrations_table_blocking, get_applied_migrations,
  get_applied_migrations_blocking, record_migration, record_migration_blocking,
};
