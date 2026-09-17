//! Shared `items` table setup for the prepared-statement-cache suites.

use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_connection::{DbConnectionBlocking, DbError};
use toolu_orm_core::value::Value;

use crate::rusqlite_rows::LabelRow;

pub const SELECT_BY_ID: &str = "SELECT label FROM items WHERE id = ?1";

pub fn create_items(conn: &RusqliteConnection) -> Result<(), DbError> {
  DbConnectionBlocking::execute_batch(
    conn,
    "CREATE TABLE items (id INTEGER PRIMARY KEY, label TEXT NOT NULL)",
  )
}

pub fn insert_item(conn: &RusqliteConnection, id: i64, label: &str) -> Result<u64, DbError> {
  DbConnectionBlocking::execute_sql(
    conn,
    "INSERT INTO items (id, label) VALUES (?1, ?2)",
    vec![Value::Integer(id), Value::Text(label.to_owned())],
  )
}

pub fn select_label(conn: &RusqliteConnection, id: i64) -> Result<Vec<LabelRow>, DbError> {
  DbConnectionBlocking::query_map(conn, SELECT_BY_ID, vec![Value::Integer(id)])
}
