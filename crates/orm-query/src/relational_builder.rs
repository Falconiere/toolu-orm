//! Type-state `RelationalQuery` built on [`RelationalSelectBuilder`].
//!
//! # Public API
//!
//! - [`RelationalQuery`]

use std::marker::PhantomData;

use toolu_orm_core::relational_row::{parse_json_array_of_arrays, parse_json_single_array};

use crate::select::relational::{RelationColumn, RelationalSelectBuilder};
use crate::tuple_append::TupleAppend;

/// Relational query with result shape in `T` (tuple that grows with each relation).
pub struct RelationalQuery<T> {
  builder: RelationalSelectBuilder,
  _marker: PhantomData<T>,
}

impl<T> RelationalQuery<T> {
  /// Inner SQL builder.
  pub fn inner_builder(&self) -> &RelationalSelectBuilder {
    &self.builder
  }

  /// Postgres SQL.
  pub fn to_sql_postgres(&self) -> String {
    self.builder.to_sql_postgres()
  }

  /// SQLite SQL.
  pub fn to_sql_sqlite(&self) -> String {
    self.builder.to_sql_sqlite()
  }

  /// Active dialect SQL.
  pub fn to_sql(&self) -> String {
    self.builder.to_sql()
  }

  /// Parse a has-many JSON column (`json_agg` / `json_group_array` shape).
  ///
  /// # Errors
  ///
  /// Forwards [`toolu_orm_core::relational_row::parse_json_array_of_arrays`] errors.
  pub fn parse_many_column(
    json: &str,
  ) -> Result<Vec<Vec<serde_json::Value>>, toolu_orm_core::error::DbCoreError> {
    parse_json_array_of_arrays(json)
  }

  /// Parse a has-one / belongs-to JSON column (`json_build_array` / null).
  ///
  /// # Errors
  ///
  /// Forwards [`toolu_orm_core::relational_row::parse_json_single_array`] errors.
  pub fn parse_one_column(
    json: &str,
  ) -> Result<Vec<serde_json::Value>, toolu_orm_core::error::DbCoreError> {
    parse_json_single_array(json)
  }

  /// Decode a relation column from SQLite's JSON text, honouring the binary
  /// columns this query declared.
  ///
  /// # Errors
  ///
  /// Forwards [`RelationalSelectBuilder::decode_relation_json`] errors.
  pub fn decode_relation_json(
    &self,
    field_name: &str,
    json: &str,
  ) -> Result<serde_json::Value, toolu_orm_core::error::DbCoreError> {
    self.builder.decode_relation_json(field_name, json)
  }

  /// Decode an already-parsed relation column (Postgres `json`, or `Value::Null`
  /// for a SQL NULL), honouring the binary columns this query declared.
  ///
  /// # Errors
  ///
  /// Forwards [`RelationalSelectBuilder::decode_relation_value`] errors.
  pub fn decode_relation_value(
    &self,
    field_name: &str,
    column: &serde_json::Value,
  ) -> Result<serde_json::Value, toolu_orm_core::error::DbCoreError> {
    self.builder.decode_relation_value(field_name, column)
  }
}

impl<Base> RelationalQuery<(Base,)> {
  /// Start a query; base row type is wrapped in a 1-tuple.
  pub fn new(table: &str, columns: &[&str]) -> Self {
    Self {
      builder: RelationalSelectBuilder::new(table, columns),
      _marker: PhantomData,
    }
  }
}

impl<T> RelationalQuery<T> {
  /// Add has-many; appends `Vec<R>`.
  pub fn with_many<R>(
    self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key_on_target: &str,
    target_columns: &[&str],
  ) -> RelationalQuery<<T as TupleAppend<Vec<R>>>::Output>
  where
    T: TupleAppend<Vec<R>>,
  {
    RelationalQuery {
      builder: self.builder.with_many(
        field_name,
        target_table,
        local_key,
        foreign_key_on_target,
        target_columns,
      ),
      _marker: PhantomData,
    }
  }

  /// Add has-many with columns that declare their transport; appends `Vec<R>`.
  pub fn with_many_columns<R>(
    self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key_on_target: &str,
    target_columns: &[RelationColumn],
  ) -> RelationalQuery<<T as TupleAppend<Vec<R>>>::Output>
  where
    T: TupleAppend<Vec<R>>,
  {
    RelationalQuery {
      builder: self.builder.with_many_columns(
        field_name,
        target_table,
        local_key,
        foreign_key_on_target,
        target_columns,
      ),
      _marker: PhantomData,
    }
  }

  /// Add belongs-to / has-one; appends `Option<R>`.
  pub fn with_one<R>(
    self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key_on_target: &str,
    target_columns: &[&str],
  ) -> RelationalQuery<<T as TupleAppend<Option<R>>>::Output>
  where
    T: TupleAppend<Option<R>>,
  {
    RelationalQuery {
      builder: self.builder.with_one(
        field_name,
        target_table,
        local_key,
        foreign_key_on_target,
        target_columns,
      ),
      _marker: PhantomData,
    }
  }

  /// Add belongs-to / has-one with columns that declare their transport;
  /// appends `Option<R>`.
  pub fn with_one_columns<R>(
    self,
    field_name: &str,
    target_table: &str,
    local_key: &str,
    foreign_key_on_target: &str,
    target_columns: &[RelationColumn],
  ) -> RelationalQuery<<T as TupleAppend<Option<R>>>::Output>
  where
    T: TupleAppend<Option<R>>,
  {
    RelationalQuery {
      builder: self.builder.with_one_columns(
        field_name,
        target_table,
        local_key,
        foreign_key_on_target,
        target_columns,
      ),
      _marker: PhantomData,
    }
  }
}
