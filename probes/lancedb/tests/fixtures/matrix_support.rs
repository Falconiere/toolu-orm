use duckdb::{Connection, Row, params_from_iter, types::Value as DuckValue};
use std::{error::Error, io};
use tempfile::TempDir;
use toolu_orm_core::{
  column::{Integer, Text},
  query_column::Column,
  value::Value,
};

use crate::lance::{LanceDependencyUnavailable, load_lance, quoted_path};

pub type TestResult = Result<(), Box<dyn Error>>;

pub const ITEM_ID: Column<Integer> = Column::new("items", "id");
pub const ITEM_GROUP: Column<Integer> = Column::new("items", "group_id");
pub const ITEM_NAME: Column<Text> = Column::new("items", "name");
pub const ITEM_SCORE: Column<Integer> = Column::new("items", "score");
pub const GROUP_ID: Column<Integer> = Column::new("groups", "id");
pub const GROUP_LABEL: Column<Text> = Column::new("groups", "label");

pub struct Fixture {
  pub connection: Connection,
  _directory: TempDir,
}

impl Fixture {
  pub fn new() -> Result<Self, Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let extension = std::env::var_os("LANCE_EXTENSION_PATH")
      .ok_or_else(|| LanceDependencyUnavailable::new("LANCE_EXTENSION_PATH is missing"))?;
    let connection = Connection::open_in_memory()?;
    load_lance(&connection, std::path::Path::new(&extension))?;
    connection.execute_batch(&format!(
      "ATTACH {} AS lance_matrix (TYPE LANCE); USE lance_matrix;\
       CREATE TABLE groups (id BIGINT, label VARCHAR);\
       CREATE TABLE items (id BIGINT, group_id BIGINT, name VARCHAR, score BIGINT);\
       INSERT INTO groups VALUES (1, 'alpha'), (2, 'beta');\
       INSERT INTO items VALUES (1, 1, 'one', 10), (2, 1, 'two', 20),\
         (3, 2, 'three', 30), (4, 99, 'orphan', 40);",
      quoted_path(directory.path())?
    ))?;
    Ok(Self {
      connection,
      _directory: directory,
    })
  }

  pub fn rows<T, F>(&self, sql: &str, params: &[Value], mapper: F) -> Result<Vec<T>, Box<dyn Error>>
  where
    F: FnMut(&Row<'_>) -> duckdb::Result<T>,
  {
    let values = duck_values(params)?;
    let mut statement = self.connection.prepare(sql)?;
    let mapped = statement.query_map(params_from_iter(values.iter()), mapper)?;
    Ok(mapped.collect::<duckdb::Result<Vec<T>>>()?)
  }

  pub fn execute(&self, sql: &str, params: &[Value]) -> Result<usize, Box<dyn Error>> {
    let values = duck_values(params)?;
    let mut statement = self.connection.prepare(sql)?;
    Ok(statement.execute(params_from_iter(values.iter()))?)
  }
}

fn duck_values(params: &[Value]) -> Result<Vec<DuckValue>, Box<dyn Error>> {
  params
    .iter()
    .map(|param| match param {
      Value::Integer(value) => Ok(DuckValue::BigInt(*value)),
      Value::Text(value) => Ok(DuckValue::Text(value.clone())),
      other => Err(io::Error::other(format!(
        "test-only Lance bind conversion does not cover {other:?}"
      ))),
    })
    .collect::<Result<Vec<_>, _>>()
    .map_err(Into::into)
}
