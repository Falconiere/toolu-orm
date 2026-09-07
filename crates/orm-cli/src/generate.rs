//! Migration file generation from schema diffs.

use std::path::Path;

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::diff;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::journal::{compute_hash, Journal};
use toolu_orm_core::schema::SchemaRegistry;
use toolu_orm_core::snapshot::Snapshot;
use toolu_orm_core::sql::generate_sql_for;

const ZERO_SNAPSHOT_PARENT: &str = "00000000-0000-0000-0000-000000000000";

/// Generates a migration file from the diff between the latest snapshot and the current schema.
///
/// # Errors
///
/// Returns `DbCoreError` if reading/writing the snapshot, journal, or migration file fails.
pub fn run_generate(
  registry: &SchemaRegistry,
  migrations_dir: &str,
  name: &str,
  dialect: Dialect,
) -> Result<Option<String>, DbCoreError> {
  let journal_path = Path::new(migrations_dir).join("_journal.json");
  let journal_path_str = journal_path.to_str().ok_or_else(|| {
    DbCoreError::JournalRead(format!("non-UTF8 path: {}", journal_path.display()))
  })?;
  let mut journal = Journal::read_from_path(journal_path_str)?;

  let old_snapshot = {
    let mut found = None;
    for entry in journal.entries.iter().rev() {
      let base = entry.name.strip_suffix(".sql").unwrap_or(&entry.name);
      let snap_name = format!("{base}.snapshot.json");
      let snap_path = Path::new(migrations_dir).join(&snap_name);
      if snap_path.exists() {
        let snap_str = snap_path.to_str().ok_or_else(|| {
          DbCoreError::SnapshotRead(format!("non-UTF8 path: {}", snap_path.display()))
        })?;
        found = Some(Snapshot::read_from_path(snap_str)?);
        break;
      }
    }
    match found {
      Some(snapshot) => snapshot,
      None if journal.entries.is_empty() => Snapshot::empty(),
      None => {
        return Err(DbCoreError::SnapshotRead(
          "no snapshot file found for any journal entry — create a snapshot for the latest migration before generating".to_owned(),
        ));
      },
    }
  };

  let ops = diff(&old_snapshot, registry);
  if ops.is_empty() {
    return Ok(None);
  }

  let sql = generate_sql_for(&ops, dialect);
  let hash = compute_hash(&sql);
  let next = journal.next_migration_number();
  let filename = format!("{next:04}_{name}.sql");
  let snapshot_name = format!("{next:04}_{name}.snapshot.json");

  let sql_path = Path::new(migrations_dir).join(&filename);
  std::fs::write(&sql_path, &sql)
    .map_err(|e| DbCoreError::MigrationWrite(format!("{}: {e}", sql_path.display())))?;

  let mut new_snapshot = Snapshot::from_registry(registry);
  new_snapshot.dialect = dialect.as_str().to_owned();
  new_snapshot.prev_id = if old_snapshot.tables.is_empty() {
    ZERO_SNAPSHOT_PARENT.to_owned()
  } else {
    old_snapshot.id.clone()
  };
  new_snapshot.enums.clone_from(&old_snapshot.enums);
  new_snapshot.meta.clone_from(&old_snapshot.meta);

  let snap_path = Path::new(migrations_dir).join(&snapshot_name);
  let snap_str = snap_path
    .to_str()
    .ok_or_else(|| DbCoreError::SnapshotWrite(format!("non-UTF8 path: {}", snap_path.display())))?;
  new_snapshot.write_to_path(snap_str)?;

  journal.add_entry(&filename, &hash);
  journal.write_to_path(journal_path_str)?;

  Ok(Some(filename))
}
