//! Postgres row-level security: the table flag and its policies.
//!
//! Declared on [`crate::table::TableDef::row_security`] and recorded in the
//! snapshot, so the diff can enable row security on a new table, create the
//! policies it declares, drop and recreate one that changed, and disable the
//! whole thing when the declaration is removed. SQLite has no equivalent; the
//! generator renders a comment there.
//!
//! Postgres semantics the types mirror: a table with row security enabled
//! and no policy is default-deny; superusers and roles with `BYPASSRLS`
//! always bypass; the owner bypasses too unless the table is `FORCE`d.

use serde::{Deserialize, Serialize};

/// The command a policy applies to — `FOR …` in `CREATE POLICY`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyCommand {
  #[default]
  All,
  Select,
  Insert,
  Update,
  Delete,
}

impl PolicyCommand {
  /// The SQL keyword.
  #[must_use]
  pub const fn as_sql(self) -> &'static str {
    match self {
      Self::All => "ALL",
      Self::Select => "SELECT",
      Self::Insert => "INSERT",
      Self::Update => "UPDATE",
      Self::Delete => "DELETE",
    }
  }

  /// True for the default, which `CREATE POLICY` leaves implicit.
  #[must_use]
  pub const fn is_all(&self) -> bool {
    matches!(self, Self::All)
  }

  /// Postgres rejects `USING` on an `INSERT` policy: there is no existing row
  /// to test.
  #[must_use]
  pub const fn accepts_using(self) -> bool {
    !matches!(self, Self::Insert)
  }

  /// Postgres rejects `WITH CHECK` on `SELECT` and `DELETE` policies: neither
  /// produces a new row.
  #[must_use]
  pub const fn accepts_with_check(self) -> bool {
    !matches!(self, Self::Select | Self::Delete)
  }
}

/// How a policy combines with the others on its table — `AS …`.
///
/// Permissive policies are OR-ed together; restrictive ones are AND-ed on top.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKind {
  #[default]
  Permissive,
  Restrictive,
}

impl PolicyKind {
  /// The SQL keyword.
  #[must_use]
  pub const fn as_sql(self) -> &'static str {
    match self {
      Self::Permissive => "PERMISSIVE",
      Self::Restrictive => "RESTRICTIVE",
    }
  }

  /// True for the default, which `CREATE POLICY` leaves implicit.
  #[must_use]
  pub const fn is_permissive(&self) -> bool {
    matches!(self, Self::Permissive)
  }
}

/// One `CREATE POLICY` as the schema declares it.
///
/// Names are per table. `using` and `with_check` hold raw SQL boolean
/// expressions, rendered verbatim inside their parentheses. `roles` is the
/// `TO` list; empty means `PUBLIC`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDef {
  pub name: String,
  #[serde(default, skip_serializing_if = "PolicyKind::is_permissive")]
  pub kind: PolicyKind,
  #[serde(default, skip_serializing_if = "PolicyCommand::is_all")]
  pub command: PolicyCommand,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub roles: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub using: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub with_check: Option<String>,
}

impl PolicyDef {
  /// A permissive policy for every command and every role, with no
  /// expression yet.
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      kind: PolicyKind::Permissive,
      command: PolicyCommand::All,
      roles: Vec::new(),
      using: None,
      with_check: None,
    }
  }

  /// Sets `AS …`.
  #[must_use]
  pub fn kind(mut self, kind: PolicyKind) -> Self {
    self.kind = kind;
    self
  }

  /// Sets `FOR …`.
  #[must_use]
  pub fn command(mut self, command: PolicyCommand) -> Self {
    self.command = command;
    self
  }

  /// Adds a role to the `TO` list.
  #[must_use]
  pub fn role(mut self, role: impl Into<String>) -> Self {
    self.roles.push(role.into());
    self
  }

  /// Sets the `USING (…)` expression.
  #[must_use]
  pub fn using(mut self, expr: impl Into<String>) -> Self {
    self.using = Some(expr.into());
    self
  }

  /// Sets the `WITH CHECK (…)` expression.
  #[must_use]
  pub fn with_check(mut self, expr: impl Into<String>) -> Self {
    self.with_check = Some(expr.into());
    self
  }
}

/// Row security on one table: enabled by being present, `FORCE`d when
/// `force` is set, with the policies it declares.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowSecurity {
  /// `ALTER TABLE … FORCE ROW LEVEL SECURITY`: the owner is subject to the
  /// policies too.
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub force: bool,
  /// The policies, in declaration order.
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub policies: Vec<PolicyDef>,
}

impl RowSecurity {
  /// Row security enabled, owner exempt, no policy: default-deny for
  /// everyone else.
  #[must_use]
  pub fn enabled() -> Self {
    Self::default()
  }

  /// Row security enabled and forced onto the owner.
  #[must_use]
  pub fn forced() -> Self {
    Self {
      force: true,
      policies: Vec::new(),
    }
  }

  /// Adds a policy.
  #[must_use]
  pub fn policy(mut self, policy: PolicyDef) -> Self {
    self.policies.push(policy);
    self
  }
}
