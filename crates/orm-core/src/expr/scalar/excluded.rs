//! `"excluded"."<column>"` — the row an upsert's INSERT half proposed.

use crate::query_column::Column;

use super::types::Scalar;

impl Scalar {
  /// The value the conflicting `INSERT` proposed for `column`, as both SQLite
  /// and Postgres spell it: `"excluded"."<name>"`.
  ///
  /// It is only in scope inside an `ON CONFLICT … DO UPDATE` clause. Used
  /// anywhere else the engine rejects the statement (`no such column:
  /// excluded.x` on SQLite, `42P01` on Postgres) and that driver error
  /// propagates unchanged — this constructor validates nothing, exactly as
  /// [`Scalar::func`] does not check that a function exists.
  ///
  /// Pair it with [`Scalar::col`], which inside the same clause names the
  /// **existing** row, to keep a stored value only when the incoming one is
  /// absent:
  ///
  /// ```
  /// use toolu_orm_core::column::Text;
  /// use toolu_orm_core::dialect::Dialect;
  /// use toolu_orm_core::expr::Scalar;
  /// use toolu_orm_core::query_column::Column;
  ///
  /// # fn main() -> Result<(), toolu_orm_core::error::DbCoreError> {
  /// const WORKSPACE: Column<Text> = Column::new("bindings", "workspace_id");
  ///
  /// let keep = Scalar::func(
  ///   "coalesce",
  ///   vec![Scalar::col(&WORKSPACE), Scalar::excluded(&WORKSPACE)],
  /// )?;
  /// let (sql, params) = keep.to_sql_fragment_for(1, Dialect::Sqlite);
  /// assert_eq!(
  ///   sql,
  ///   r#"coalesce("bindings"."workspace_id", "excluded"."workspace_id")"#
  /// );
  /// assert!(params.is_empty());
  /// # Ok(())
  /// # }
  /// ```
  #[must_use]
  pub fn excluded<T>(column: &Column<T>) -> Self {
    // An embedded double quote doubles — the only escape a delimited
    // identifier has, in SQLite and Postgres alike, and the same one the
    // alias module applies to the identifiers it renders. `column.name` is a
    // `&'static str` written in Rust source, so for a legal identifier this
    // is byte-identical to a bare wrap.
    Self::sql(format!(
      r#""excluded"."{}""#,
      column.name.replace('"', "\"\"")
    ))
  }
}
