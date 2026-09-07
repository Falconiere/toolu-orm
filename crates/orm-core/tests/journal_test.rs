use toolu_orm_core::journal::{compute_hash, Journal};

#[test]
fn test_empty_journal() {
  let journal = Journal::empty();
  assert_eq!(journal.version, 1);
  assert!(journal.entries.is_empty());
}

#[test]
fn test_journal_add_entry() -> Result<(), Box<dyn std::error::Error>> {
  let mut journal = Journal::empty();
  journal.add_entry("0001_initial.sql", "sha256:abc123");
  assert_eq!(journal.entries.len(), 1);
  let entry = journal.entries.first().ok_or("entry must exist")?;
  assert_eq!(entry.idx, 0);
  assert_eq!(entry.name, "0001_initial.sql");
  assert_eq!(entry.hash, "sha256:abc123");
  Ok(())
}

#[test]
fn test_journal_next_number() {
  let mut journal = Journal::empty();
  journal.add_entry("0001_initial.sql", "hash1");
  journal.add_entry("0002_add_users.sql", "hash2");
  assert_eq!(journal.next_migration_number(), 3);
}

#[test]
fn test_journal_round_trip_file() -> Result<(), Box<dyn std::error::Error>> {
  let dir = tempfile::tempdir()?;
  let path = dir.path().join("_journal.json");
  let path_str = path.to_str().ok_or("path to_str failed")?;

  let mut journal = Journal::empty();
  journal.add_entry("0001_initial.sql", "hash1");
  journal.write_to_path(path_str)?;

  let loaded = Journal::read_from_path(path_str)?;
  assert_eq!(loaded.entries.len(), 1);
  let entry = loaded.entries.first().ok_or("entry must exist")?;
  assert_eq!(entry.name, "0001_initial.sql");
  Ok(())
}

#[test]
fn test_journal_read_missing_file_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
  let journal = Journal::read_from_path("/nonexistent/_journal.json")?;
  assert!(journal.entries.is_empty());
  Ok(())
}

#[test]
fn test_compute_file_hash() {
  let hash = compute_hash("CREATE TABLE users (id TEXT);");
  assert!(hash.starts_with("sha256:"));
  assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
}

#[test]
fn test_journal_latest_snapshot_path() {
  let mut journal = Journal::empty();
  journal.add_entry("0001_initial.sql", "hash1");
  assert_eq!(
    journal.latest_snapshot_name_owned(),
    Some("0001_initial.snapshot.json".to_owned())
  );
}
