//! DDL for `CREATE VIRTUAL TABLE … USING <module>(<args>)`.
//!
//! Virtual tables carry no column types, no constraints and no `STRICT`
//! clause: everything inside the parentheses is the module's own argument
//! syntax, already rendered into [`crate::table::TableKind::Virtual`].

use crate::dialect::Dialect;

/// `CREATE VIRTUAL TABLE IF NOT EXISTS "<name>" USING "<module>"(<args>);`
///
/// The table and module names are quoted identifiers with their embedded
/// quotes doubled, so neither can end the statement early. With no arguments
/// the parentheses are omitted, which is what SQLite expects for modules that
/// take none.
pub(crate) fn create_virtual_table_sql(name: &str, module: &str, args: &[String]) -> String {
  let name = quote_ident(name);
  let module = quote_ident(module);
  if args.is_empty() {
    return format!("CREATE VIRTUAL TABLE IF NOT EXISTS {name} USING {module};");
  }
  format!(
    "CREATE VIRTUAL TABLE IF NOT EXISTS {name} USING {module}({});",
    args.join(", ")
  )
}

/// A double-quoted identifier; an embedded double quote doubles.
fn quote_ident(ident: &str) -> String {
  format!("\"{}\"", ident.replace('"', "\"\""))
}

/// Postgres has no virtual tables. The migration chunk says so instead of
/// silently dropping the table, mirroring how enums and SQLite-only
/// constraints are reported on the other dialect.
pub(crate) fn unsupported_dialect_comment(name: &str, module: &str, dialect: Dialect) -> String {
  format!(
    "-- virtual table {} USING {} is SQLite-only; skipped for {}",
    quote_ident(name),
    quote_ident(module),
    dialect.as_str()
  )
}
