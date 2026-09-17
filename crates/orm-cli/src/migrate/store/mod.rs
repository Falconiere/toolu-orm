//! What `_migrations` holds: the recorded row, the lookups over it, the insert
//! that adds one, and the table itself.

mod applied;
mod lookup;
mod record;
mod table;

pub(crate) use applied::AppliedMigration;
pub(crate) use lookup::{get_applied_history, get_applied_history_blocking};
pub use lookup::{get_applied_migrations, get_applied_migrations_blocking};
pub use record::{record_migration, record_migration_blocking};
pub use table::{ensure_migrations_table, ensure_migrations_table_blocking};
