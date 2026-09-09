//! DDL for `CREATE VIRTUAL TABLE … USING <module>(<args>)`.
//!
//! Virtual tables carry no column types, no constraints and no `STRICT`
//! clause: everything inside the parentheses is the module's own argument
//! syntax, already rendered into [`crate::table::TableKind::Virtual`].

use crate::dialect::Dialect;
use crate::table::{TableDef, TableKind};

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

/// Drop + recreate an FTS5 table, then rebuild from its external content table.
pub(crate) fn recreate_fts5_from_content_sql(table: &TableDef, dialect: Dialect) -> String {
  match dialect {
    Dialect::Sqlite => {
      let create = match &table.kind {
        TableKind::Virtual { module, args } => create_virtual_table_sql(&table.name, module, args),
        TableKind::Ordinary => create_virtual_table_sql(&table.name, "fts5", &[]),
      };
      let name = &table.name;
      format!(
        "DROP TABLE IF EXISTS \"{name}\";\n\
         --> statement-breakpoint\n\
         {create}\n\
         --> statement-breakpoint\n\
         INSERT INTO \"{name}\"(\"{name}\") VALUES('rebuild');"
      )
    },
    Dialect::Postgres => match table.kind.module() {
      Some(module) => unsupported_dialect_comment(&table.name, module, dialect),
      None => unsupported_dialect_comment(&table.name, "fts5", dialect),
    },
  }
}

/// A double-quoted identifier; an embedded double quote doubles.
fn quote_ident(ident: &str) -> String {
  format!("\"{}\"", ident.replace('"', "\"\""))
}

/// Postgres has no virtual tables. The migration chunk says so instead of
/// silently dropping the table, mirroring how enums and SQLite-only
/// constraints are reported on the other dialect.
///
/// A `--` comment ends at the newline, so a name carrying one would put the
/// rest of itself back into the migration as SQL; both names are folded onto
/// this single line.
pub(crate) fn unsupported_dialect_comment(name: &str, module: &str, dialect: Dialect) -> String {
  format!(
    "-- virtual table {} USING {} is SQLite-only; skipped for {}",
    single_line(&quote_ident(name)),
    single_line(&quote_ident(module)),
    dialect.as_str()
  )
}

/// The text with its line breaks turned into spaces.
fn single_line(text: &str) -> String {
  text.replace(['\n', '\r'], " ")
}
