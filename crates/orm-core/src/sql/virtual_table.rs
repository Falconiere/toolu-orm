//! DDL for `CREATE VIRTUAL TABLE … USING <module>(<args>)`.
//!
//! Virtual tables carry no column types, no constraints and no `STRICT`
//! clause: everything inside the parentheses is the module's own argument
//! syntax, already rendered into [`crate::table::TableKind::Virtual`].

use crate::dialect::Dialect;

/// `CREATE VIRTUAL TABLE IF NOT EXISTS "<name>" USING <module>(<args>);`
///
/// With no arguments the parentheses are omitted, which is what SQLite
/// expects for modules that take none.
pub(crate) fn create_virtual_table_sql(name: &str, module: &str, args: &[String]) -> String {
  if args.is_empty() {
    return format!("CREATE VIRTUAL TABLE IF NOT EXISTS \"{name}\" USING {module};");
  }
  format!(
    "CREATE VIRTUAL TABLE IF NOT EXISTS \"{name}\" USING {module}({});",
    args.join(", ")
  )
}

/// Postgres has no virtual tables. The migration chunk says so instead of
/// silently dropping the table, mirroring how enums and SQLite-only
/// constraints are reported on the other dialect.
pub(crate) fn unsupported_dialect_comment(name: &str, module: &str, dialect: Dialect) -> String {
  format!(
    "-- virtual table \"{name}\" USING {module} is SQLite-only; skipped for {}",
    dialect.as_str()
  )
}
