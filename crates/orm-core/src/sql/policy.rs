//! Row-level security statements: the table flags and `CREATE` / `DROP POLICY`.
//!
//! Postgres only. SQLite has no row security at all, so each operation renders
//! as a `--` comment there — the same treatment enums and foreign-key
//! constraints get — and the migration file shows what was skipped.

use crate::dialect::Dialect;
use crate::policy::PolicyDef;

/// Separator between several statements rendered for one operation.
const BREAKPOINT: &str = "\n\n--> statement-breakpoint\n\n";

/// Role keywords `CREATE POLICY … TO` accepts unquoted; anything else is a
/// role name and is quoted.
const ROLE_KEYWORDS: [&str; 4] = ["PUBLIC", "CURRENT_ROLE", "CURRENT_USER", "SESSION_USER"];

/// Both flags to exactly the requested state, as two statements.
pub(crate) fn alter_row_level_security_sql(
  table: &str,
  enabled: bool,
  force: bool,
  dialect: Dialect,
) -> String {
  match dialect {
    Dialect::Postgres => {
      let enable = if enabled { "ENABLE" } else { "DISABLE" };
      let force = if force { "FORCE" } else { "NO FORCE" };
      format!(
        "ALTER TABLE \"{table}\" {enable} ROW LEVEL SECURITY;{BREAKPOINT}\
         ALTER TABLE \"{table}\" {force} ROW LEVEL SECURITY;"
      )
    },
    Dialect::Sqlite => format!(
      "-- row level security on \"{table}\" (Postgres only; SQLite has no row-level security)"
    ),
  }
}

/// `CREATE POLICY`, with every clause that differs from its default.
pub(crate) fn create_policy_sql(table: &str, policy: &PolicyDef, dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => {
      let mut sql = format!("CREATE POLICY \"{}\" ON \"{table}\"", policy.name);
      if !policy.kind.is_permissive() {
        sql.push_str(&format!(" AS {}", policy.kind.as_sql()));
      }
      if !policy.command.is_all() {
        sql.push_str(&format!(" FOR {}", policy.command.as_sql()));
      }
      if !policy.roles.is_empty() {
        let roles: Vec<String> = policy.roles.iter().map(|r| role_sql(r)).collect();
        sql.push_str(&format!(" TO {}", roles.join(", ")));
      }
      if let Some(using) = &policy.using {
        sql.push_str(&format!(" USING ({using})"));
      }
      if let Some(check) = &policy.with_check {
        sql.push_str(&format!(" WITH CHECK ({check})"));
      }
      sql.push(';');
      sql
    },
    Dialect::Sqlite => format!(
      "-- policy \"{}\" on \"{table}\" (Postgres only; SQLite has no row-level security)",
      policy.name
    ),
  }
}

/// `DROP POLICY IF EXISTS`.
pub(crate) fn drop_policy_sql(table: &str, name: &str, dialect: Dialect) -> String {
  match dialect {
    Dialect::Postgres => format!("DROP POLICY IF EXISTS \"{name}\" ON \"{table}\";"),
    Dialect::Sqlite => format!(
      "-- drop policy \"{name}\" on \"{table}\" (Postgres only; SQLite has no row-level security)"
    ),
  }
}

/// A role in a `TO` list: the four special keywords bare, any other name
/// quoted as an identifier.
fn role_sql(role: &str) -> String {
  let upper = role.to_ascii_uppercase();
  if ROLE_KEYWORDS.contains(&upper.as_str()) {
    upper
  } else {
    format!("\"{role}\"")
  }
}
