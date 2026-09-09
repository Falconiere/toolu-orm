mod apply;
mod baseline;
mod ddl;
pub(crate) mod embedded;
mod error;
mod missing_extension;
mod pending;
mod run;
mod store;
mod transaction;

pub use baseline::{mark_applied, mark_applied_through};
pub use ddl::migrations_table_ddl;
pub use embedded::{
  mark_applied_embedded, mark_applied_through_embedded, run_migrate_embedded, EmbeddedMigration,
};
pub use error::MigrateError;
pub use run::run_migrate;
pub use store::{ensure_migrations_table, get_applied_migrations, record_migration};
