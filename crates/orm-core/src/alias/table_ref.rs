//! [`TableRef`] — what a `FROM` / `JOIN` slot names, optionally under an alias.

use crate::dialect::Dialect;
use crate::error::DbCoreError;
use crate::expr::function_name::is_valid_function_name;
use crate::query_column::Column;
use crate::value::Value;

use super::aliased_column::AliasedColumn;
use super::quoting::quote_ident;
use super::table_function::{render_call, TableSource};

/// A source named in a `FROM` or `JOIN` slot, with an optional alias.
///
/// One type covers all three: a schema table, a common table expression (which
/// SQL references by a plain identifier, so it needs nothing extra), and a
/// table-valued function call carrying bound arguments.
///
/// Keeping the alias out of the name is what makes a self-join expressible:
/// `TableRef::aliased("memories", "old")` renders `"memories" AS "old"`, where
/// the whole string in one identifier — `"memories old"` — names nothing.
///
/// `Eq` is deliberately not derived: a function source holds [`Value`]
/// arguments, and `Value::Real` holds an `f64`.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRef {
  table: String,
  alias: Option<String>,
  source: TableSource,
}

impl TableRef {
  /// The relation under its own name — a table, or a CTE declared by
  /// `SelectBuilder::with`.
  pub fn new(table: impl Into<String>) -> Self {
    Self {
      table: table.into(),
      alias: None,
      source: TableSource::Relation,
    }
  }

  /// The relation under `alias`, which is what its columns are qualified by.
  pub fn aliased(table: impl Into<String>, alias: impl Into<String>) -> Self {
    Self {
      table: table.into(),
      alias: Some(alias.into()),
      source: TableSource::Relation,
    }
  }

  /// A table-valued function call: `json_each(?1)`, `pragma_table_info(?1)`,
  /// `regexp_split_to_table($1, $2)`.
  ///
  /// Arguments are bound, never interpolated; the *name* is validated instead,
  /// like [`Scalar::func`](crate::expr::Scalar::func)'s. No dialect
  /// translation happens — `json_each` is SQLite's, and naming it for Postgres
  /// is the caller's error.
  ///
  /// ```
  /// use toolu_orm_core::alias::TableRef;
  /// use toolu_orm_core::dialect::Dialect;
  /// use toolu_orm_core::value::Value;
  ///
  /// # fn main() -> Result<(), toolu_orm_core::error::DbCoreError> {
  /// let seeds = TableRef::function("json_each", vec![Value::Text("[1]".to_owned())])?
  ///   .with_alias("seeds");
  /// let (sql, params) = seeds.to_sql_fragment_for(1, Dialect::Sqlite);
  /// assert_eq!(sql, r#"json_each(?1) AS "seeds""#);
  /// assert_eq!(params.len(), 1);
  /// assert!(TableRef::function("drop table t; --", Vec::new()).is_err());
  /// # Ok(())
  /// # }
  /// ```
  ///
  /// # Errors
  ///
  /// [`DbCoreError::InvalidTableFunction`] when `name` is empty, starts with a
  /// digit, or holds anything but ASCII letters, digits and underscores.
  pub fn function(name: &str, args: Vec<Value>) -> Result<Self, DbCoreError> {
    if !is_valid_function_name(name) {
      return Err(DbCoreError::InvalidTableFunction {
        name: name.to_owned(),
      });
    }
    Ok(Self {
      table: name.to_owned(),
      alias: None,
      source: TableSource::Function(args),
    })
  }

  /// The same source under `alias`, which its columns are then qualified by.
  ///
  /// The companion of [`Self::aliased`] for the sources that have no
  /// two-argument constructor.
  pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
    self.alias = Some(alias.into());
    self
  }

  /// The base name, never the alias — the table, the CTE, or the function.
  ///
  /// This is the name an error message should carry, because it is what the
  /// caller named.
  pub fn table(&self) -> &str {
    &self.table
  }

  /// The alias, when there is one.
  pub fn alias(&self) -> Option<&str> {
    self.alias.as_deref()
  }

  /// What columns of this source are qualified by: the alias, else the name.
  ///
  /// SQL hides the original name once a source is aliased, so this is the only
  /// qualifier the rest of the statement may use.
  pub fn qualifier(&self) -> &str {
    self.alias.as_deref().unwrap_or(&self.table)
  }

  /// The source and the values its arguments bind, first placeholder at
  /// `start`.
  ///
  /// A relation binds nothing and returns an empty vector; a function returns
  /// its arguments in bind order. A `FROM` slot is a clause like any other,
  /// numbering from what the clauses rendered before it already emitted.
  pub fn to_sql_fragment_for(&self, start: usize, dialect: Dialect) -> (String, Vec<Value>) {
    let (base, params) = match &self.source {
      TableSource::Relation => (quote_ident(&self.table), Vec::new()),
      TableSource::Function(args) => render_call(&self.table, args, start, dialect),
    };
    match &self.alias {
      Some(alias) => (format!("{base} AS {}", quote_ident(alias)), params),
      None => (base, params),
    }
  }

  /// [`Self::to_sql_fragment_for`] at `start = 1` against [`Dialect::CURRENT`],
  /// dropping the values.
  ///
  /// Byte-identical to what it always returned for a relation source. A
  /// function source renders a self-consistent *standalone* call; a builder
  /// splicing one after other clauses uses [`Self::to_sql_fragment_for`].
  pub fn to_sql(&self) -> String {
    self.to_sql_fragment_for(1, Dialect::CURRENT).0
  }

  /// `column` of this source, qualified by [`TableRef::qualifier`].
  ///
  /// The marker type rides along, so an [`AliasedColumn`] keeps the same typed
  /// predicates the original [`Column`] had.
  pub fn column<T>(&self, column: &Column<T>) -> AliasedColumn<T> {
    AliasedColumn::new(self.qualifier(), column.name)
  }
}

impl From<&str> for TableRef {
  fn from(table: &str) -> Self {
    Self::new(table)
  }
}

impl From<String> for TableRef {
  fn from(table: String) -> Self {
    Self::new(table)
  }
}

/// So a builder can take `&table_ref` without the caller spelling out a clone
/// at each of the several slots one alias is named in.
impl From<&TableRef> for TableRef {
  fn from(table: &TableRef) -> Self {
    table.clone()
  }
}
