//! Table-level `FOREIGN KEY` clauses and the Postgres statement that attaches
//! one to an existing table.

use crate::snapshot::ForeignKeyDef;

/// `FOREIGN KEY ("a", "b") REFERENCES "t" ("x", "y")`, then its actions.
pub(crate) fn foreign_key_clause(fk: &ForeignKeyDef) -> String {
  let mut s = format!(
    "FOREIGN KEY ({}) REFERENCES \"{}\" ({})",
    quoted_csv(&fk.columns),
    fk.references_table,
    quoted_csv(&fk.references_columns)
  );
  if let Some(a) = fk.on_delete {
    s.push_str(&format!(" ON DELETE {}", a.as_sql()));
  }
  if let Some(a) = fk.on_update {
    s.push_str(&format!(" ON UPDATE {}", a.as_sql()));
  }
  s
}

/// `ALTER TABLE … ADD CONSTRAINT "<name>" FOREIGN KEY …;` (Postgres).
pub(crate) fn add_constraint_sql(table: &str, fk: &ForeignKeyDef) -> String {
  format!(
    "ALTER TABLE \"{table}\" ADD CONSTRAINT \"{}\" {};",
    fk.name,
    foreign_key_clause(fk)
  )
}

/// `"a", "b"` — identifiers quoted and comma-joined.
fn quoted_csv(names: &[String]) -> String {
  names
    .iter()
    .map(|c| format!("\"{c}\""))
    .collect::<Vec<_>>()
    .join(", ")
}
