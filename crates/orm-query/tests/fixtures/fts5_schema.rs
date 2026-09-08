//! The FTS5 schema and seed rows both driver lanes search, declared the way a
//! consumer would — through [`Fts5Table`] — and rendered by the real DDL
//! generator, so the table these queries read is the table #18's write side
//! creates.

use toolu_orm_core::column::ColumnType;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::diff::Operation;
use toolu_orm_core::fts5::Fts5Table;
use toolu_orm_core::sql::generate_sql_for;

/// Column order of `memory_fts`, and therefore the order `bm25` weights and
/// `snippet` / `highlight` column indexes follow.
pub const FTS_COLUMNS: [&str; 3] = ["memory_id", "body", "tags"];

/// Seeded rows: `(memory_id, body, tags, deleted)`.
///
/// Chosen so the two halves of the index disagree — `m2` carries the term in
/// `body`, `m3` in `tags` — which is what makes a weight change observable in
/// the returned order. `m1` proves the porter tokenizer (`running` answers
/// `run`), `m4` never matches, and `m4` is also the soft-deleted row.
pub const ROWS: [(&str, &str, &str, bool); 4] = [
  ("m1", "the zebrafish keeps running fast", "bio notes", false),
  ("m2", "a marathon runner trains daily", "sport", false),
  ("m3", "shoes and laces", "runner gear", false),
  ("m4", "completely unrelated text", "none", true),
];

/// `CREATE VIRTUAL TABLE … USING fts5(…)` from the real generator.
pub fn create_fts_sql() -> String {
  let table = Fts5Table::new("memory_fts")
    .unindexed_column("memory_id", ColumnType::Text)
    .column("body", ColumnType::Text)
    .column("tags", ColumnType::Text)
    .tokenize("porter unicode61 remove_diacritics 2")
    .build();
  generate_sql_for(&[Operation::CreateTable { table }], Dialect::Sqlite)
}

/// The ordinary table the search joins against, carrying the soft-delete flag.
pub const CREATE_MEMORIES_SQL: &str =
  "CREATE TABLE \"memories\" (id TEXT PRIMARY KEY, deleted_at INTEGER)";
