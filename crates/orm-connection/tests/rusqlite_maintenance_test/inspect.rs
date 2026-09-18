//! Typed pragma reads: what `quick_check` reports, and what the page counts say.

use toolu_orm_connection::SqliteMaintenance;

use crate::temp_db_dir::{TempDbDir, TestResult, seeded_db};

/// `is_ok()` discriminates. A database whose stored rows violate a `NOT NULL`
/// column its own schema declares is reported, in SQLite's words, as a report
/// rather than as an error.
#[test]
fn quick_check_reports_a_real_integrity_problem() -> TestResult {
  let dir = TempDbDir::new("integrity")?;
  let path = dir.file("broken.db");
  {
    // Built and closed on its own connection: the connection that rewrites
    // `sqlite_schema` still holds the old schema cached, so it would report the
    // database as healthy.
    let builder = rusqlite::Connection::open(&path)?;
    builder.execute_batch(
      "CREATE TABLE q (a INTEGER PRIMARY KEY, b TEXT, c TEXT);
       INSERT INTO q (b, c) VALUES (NULL, NULL);",
    )?;
    builder.execute_batch("PRAGMA writable_schema = ON")?;
    // Two columns, so the report carries two rows and its Display has something
    // to separate.
    builder.execute(
      "UPDATE sqlite_schema SET sql = ?1 WHERE type = 'table' AND name = 'q'",
      ["CREATE TABLE q (a INTEGER PRIMARY KEY, b TEXT NOT NULL, c TEXT NOT NULL)"],
    )?;
    builder.execute_batch("PRAGMA writable_schema = OFF")?;
  }

  let conn = rusqlite::Connection::open_in_memory()?;
  let broken = conn.attach_database(&path, "broken")?;
  let report = broken.quick_check()?;

  assert!(
    !report.is_ok(),
    "a NOT NULL violation must not read as healthy: {report}"
  );
  // Asserted by structure and by the columns named, rather than against
  // SQLite's exact prose, which is not a contract this crate controls.
  assert_eq!(
    report.messages().len(),
    2,
    "one row per violated column, got {:?}",
    report.messages()
  );
  for (message, column) in report.messages().iter().zip(["q.b", "q.c"]) {
    assert!(
      message.contains("NULL value in") && message.contains(column),
      "the report must name {column}, got {message:?}"
    );
  }
  assert_eq!(
    report.to_string(),
    report.messages().join("; "),
    "a multi-problem report must render every problem it holds"
  );
  // The reporting connection's own database is still fine. This is the reason
  // the trait names `main` explicitly: a schema-less `PRAGMA quick_check`
  // checks every attached database, so an unqualified call here would hand the
  // attachment's problems back as if they were the caller's own.
  assert!(
    conn.quick_check()?.is_ok(),
    "the main database must be reported on its own, not together with the attachment"
  );
  Ok(())
}

/// The page numbers are real: they multiply out to the file's length on disk and
/// they grow when the database does.
#[test]
fn attached_page_counts_match_the_file_on_disk() -> TestResult {
  let dir = TempDbDir::new("stats")?;
  let path = dir.file("stats.db");
  {
    let builder = rusqlite::Connection::open(&path)?;
    // Set before the first table, and never changed afterwards; journal_mode is
    // left at SQLite's default rollback journal, under which page_count *
    // page_size is exactly the main file's length once writes have committed.
    builder.execute_batch("PRAGMA page_size = 4096; CREATE TABLE t (a TEXT NOT NULL);")?;
    for row in 0..200 {
      builder.execute("INSERT INTO t (a) VALUES (?1)", [format!("row-{row}")])?;
    }
  }

  let conn = rusqlite::Connection::open_in_memory()?;
  let stats_db = conn.attach_database(&path, "stats")?;

  assert_eq!(stats_db.page_size()?, 4096);
  let stats = stats_db.storage_stats()?;
  assert_eq!(stats.page_count, stats_db.page_count()?);
  assert_eq!(stats.bytes(), stats.page_count * 4096);
  assert_eq!(
    stats.bytes(),
    i64::try_from(std::fs::metadata(&path)?.len())?,
    "page_count * page_size must be the file's real length"
  );

  for row in 200..600 {
    conn.execute(
      "INSERT INTO stats.t (a) VALUES (?1)",
      [format!("row-{row}")],
    )?;
  }
  assert!(
    stats_db.page_count()? > stats.page_count,
    "the page count must follow the database as it grows"
  );

  // An attachment for a file that does not exist yet is an empty database.
  let empty = conn.attach_database(&dir.file("absent.db"), "empty")?;
  assert_eq!(empty.page_count()?, 0);
  Ok(())
}

/// The same reads on the main database, through the trait rather than a guard.
#[test]
fn main_database_reads_go_through_the_same_pragmas() -> TestResult {
  let dir = TempDbDir::new("main-stats")?;
  let conn = seeded_db(&dir.file("main.db"), 4)?;

  let stats = conn.storage_stats()?;
  assert_eq!(stats.page_count, conn.page_count()?);
  assert_eq!(stats.page_size, conn.page_size()?);
  assert!(stats.page_count > 0, "a seeded database has pages");
  assert_eq!(stats.bytes(), stats.page_count * stats.page_size);
  assert!(conn.quick_check()?.is_ok());
  Ok(())
}
