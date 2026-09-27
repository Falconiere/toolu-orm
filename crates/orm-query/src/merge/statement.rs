//! Bound native MERGE rendering with target-typed source columns.

use super::{Matched, MergeBuilder, NotMatched};
use toolu_orm_connection::DbError;
use toolu_orm_core::{alias::quote_ident, dialect::Dialect, value::Value};

impl MergeBuilder {
  /// Render a validated native statement with row-major parameter order.
  ///
  /// # Errors
  /// Returns the same preflight errors as [`Self::validate_for`].
  pub fn to_sql_for(&self, dialect: Dialect) -> Result<(String, Vec<Value>), DbError> {
    self.validate_for(dialect)?;
    let target = self.target_sql();
    let columns: Vec<_> = self.columns.iter().map(|name| quote_ident(name)).collect();
    let names = columns.join(", ");
    let mut params = Vec::new();
    let mut rows = Vec::new();
    for row in &self.rows {
      let mut binds = Vec::new();
      for value in row {
        params.push(value.clone());
        binds.push(dialect.param(params.len()));
      }
      rows.push(format!("SELECT {}", binds.join(", ")));
    }
    // The empty projection anchors PostgreSQL parameter types (even all-NULL
    // columns) to the target schema. It reads no rows and adds no extra statement.
    let source = format!(
      "SELECT {names} FROM {target} WHERE false UNION ALL {}",
      rows.join(" UNION ALL ")
    );
    let keys: Vec<_> = self
      .keys
      .iter()
      .map(|key| {
        let key = quote_ident(key);
        format!("\"toolu_target\".{key} = \"toolu_source\".{key}")
      })
      .collect();
    let mut sql = format!(
      "MERGE INTO {target} AS \"toolu_target\" USING ({source}) AS \"toolu_source\" ON {}",
      keys.join(" AND ")
    );
    if self.matched == Some(Matched::Update) {
      let assignments: Vec<_> = self
        .columns
        .iter()
        .filter(|name| !self.keys.contains(name))
        .map(|name| {
          let column = quote_ident(name);
          format!("{column} = \"toolu_source\".{column}")
        })
        .collect();
      sql.push_str(&format!(
        " WHEN MATCHED THEN UPDATE SET {}",
        assignments.join(", ")
      ));
    }
    if self.not_matched == Some(NotMatched::Insert) {
      let values: Vec<_> = columns
        .iter()
        .map(|name| format!("\"toolu_source\".{name}"))
        .collect();
      sql.push_str(&format!(
        " WHEN NOT MATCHED THEN INSERT ({names}) VALUES ({})",
        values.join(", ")
      ));
    }
    Ok((sql, params))
  }
}
