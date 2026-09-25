#![cfg(feature = "lancedb")]

use std::{error::Error, path::Path};
use toolu_orm_connection::{
  LanceColumn, LanceColumnType, LanceConnection, LanceNamespace, LanceNamespaceError,
};

fn extension() -> Result<std::path::PathBuf, Box<dyn Error>> {
  Ok(
    std::env::var_os("LANCE_EXTENSION_PATH")
      .map(std::path::PathBuf::from)
      .ok_or("LANCE_EXTENSION_PATH is required for real Lance namespace tests")?,
  )
}

fn attach(directory: &Path, name: &str) -> Result<LanceNamespace, Box<dyn Error>> {
  Ok(LanceConnection::open(extension()?)?.attach(directory, name)?)
}

fn columns() -> [LanceColumn<'static>; 2] {
  [
    LanceColumn {
      name: "id",
      data_type: LanceColumnType::BigInt,
    },
    LanceColumn {
      name: "label",
      data_type: LanceColumnType::Varchar,
    },
  ]
}

fn assert_invalid_table_name(namespace: &LanceNamespace, directory: &Path, name: &str) {
  assert!(matches!(
    namespace.create_table(name, &columns()),
    Err(LanceNamespaceError::InvalidIdentifier(rejected)) if rejected == name
  ));
  assert!(!directory.join(format!("{name}.lance")).exists());
}

#[test]
fn fresh_connection_reopens_persisted_table_under_unqualified_name() -> Result<(), Box<dyn Error>> {
  let root = tempfile::tempdir()?;
  let directory = root.path().join("quote's lance namespace");
  std::fs::create_dir(&directory)?;

  {
    let namespace = attach(&directory, "first_catalog")?;
    namespace.create_table("items", &columns())?;
    namespace.connection().execute(
      "INSERT INTO items VALUES (?1, ?2)",
      duckdb::params![7_i64, "persisted"],
    )?;
    let label: String = namespace.connection().query_row(
      "SELECT label FROM items WHERE id = ?1",
      [7_i64],
      |row| row.get(0),
    )?;
    assert_eq!(label, "persisted");
  }

  assert!(directory.join("items.lance").exists());
  let reopened = attach(&directory, "second_catalog")?;
  reopened.open_table("items")?;
  assert_eq!(reopened.list_tables()?, vec!["items"]);
  let label: String =
    reopened
      .connection()
      .query_row("SELECT label FROM items WHERE id = ?1", [7_i64], |row| {
        row.get(0)
      })?;
  assert_eq!(label, "persisted");
  Ok(())
}

#[test]
fn duplicate_create_and_missing_table_errors_preserve_existing_rows() -> Result<(), Box<dyn Error>>
{
  let directory = tempfile::tempdir()?;
  let namespace = attach(directory.path(), "data")?;
  namespace.create_table("items", &columns())?;
  namespace.connection().execute(
    "INSERT INTO items VALUES (?1, ?2)",
    duckdb::params![1_i64, "keep"],
  )?;

  let duplicate = namespace
    .create_table("items", &columns())
    .err()
    .ok_or("duplicate table creation succeeded")?;
  assert!(matches!(
    duplicate,
    LanceNamespaceError::TableAlreadyExists(ref name) if name == "items"
  ));
  let missing_open = namespace
    .open_table("absent")
    .err()
    .ok_or("opening an absent table succeeded")?;
  assert!(matches!(
    missing_open,
    LanceNamespaceError::TableNotFound(ref name) if name == "absent"
  ));
  let missing_drop = namespace
    .drop_table("absent")
    .err()
    .ok_or("dropping an absent table succeeded")?;
  assert!(matches!(
    missing_drop,
    LanceNamespaceError::TableNotFound(ref name) if name == "absent"
  ));

  let count: i64 = namespace.connection().query_row(
    "SELECT count(*) FROM items WHERE label = 'keep'",
    [],
    |row| row.get(0),
  )?;
  assert_eq!(count, 1);
  Ok(())
}

#[test]
fn drop_removes_only_selected_table_after_reopen() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  {
    let namespace = attach(directory.path(), "data")?;
    namespace.create_table("remove_me", &columns())?;
    namespace.create_table("keep_me", &columns())?;
    namespace.connection().execute(
      "INSERT INTO keep_me VALUES (?1, ?2)",
      duckdb::params![2_i64, "safe"],
    )?;
    namespace.drop_table("remove_me")?;
    assert_eq!(namespace.list_tables()?, vec!["keep_me"]);
  }

  let reopened = attach(directory.path(), "data_again")?;
  assert_eq!(reopened.list_tables()?, vec!["keep_me"]);
  let count: i64 = reopened
    .connection()
    .query_row("SELECT count(*) FROM keep_me", [], |row| row.get(0))?;
  assert_eq!(count, 1);
  assert!(!directory.path().join("remove_me.lance").exists());
  Ok(())
}

#[test]
fn path_and_identifier_inputs_cannot_execute_extra_sql() -> Result<(), Box<dyn Error>> {
  let root = tempfile::tempdir()?;
  let missing = root.path().join("missing");
  let error = LanceConnection::open(extension()?)?
    .attach(&missing, "data")
    .err()
    .ok_or("missing directory attached")?;
  assert!(matches!(error, LanceNamespaceError::InvalidPath(_)));
  assert!(!missing.exists());

  let error = LanceConnection::open(extension()?)?
    .attach(root.path(), "bad/name")
    .err()
    .ok_or("namespace with a separator attached")?;
  assert!(matches!(error, LanceNamespaceError::InvalidIdentifier(_)));

  let namespace = attach(root.path(), "safe_catalog")?;
  namespace.create_table("keep", &columns())?;
  namespace.connection().execute(
    "INSERT INTO keep VALUES (?1, ?2)",
    duckdb::params![1_i64, "safe"],
  )?;
  let quoted_name = "evil\"; DROP TABLE keep; --";
  assert_invalid_table_name(&namespace, root.path(), quoted_name);
  assert_invalid_table_name(&namespace, root.path(), "bad/name");
  let invalid = namespace
    .open_table("bad\0name")
    .err()
    .ok_or("table name with NUL was accepted")?;
  assert!(matches!(invalid, LanceNamespaceError::InvalidIdentifier(_)));
  let invalid = namespace
    .create_table(
      "bad_column",
      &[LanceColumn {
        name: "bad/name",
        data_type: LanceColumnType::BigInt,
      }],
    )
    .err()
    .ok_or("column name with a separator was accepted")?;
  assert!(matches!(invalid, LanceNamespaceError::InvalidIdentifier(_)));

  assert_invalid_table_name(&namespace, root.path(), "9leading");
  assert_invalid_table_name(&namespace, root.path(), "café");

  namespace.open_table("keep")?;
  let label: String =
    namespace
      .connection()
      .query_row("SELECT label FROM keep WHERE id = 1", [], |row| row.get(0))?;
  assert_eq!(label, "safe");
  assert!(
    !namespace
      .list_tables()?
      .iter()
      .any(|name| name == "bad_column")
  );
  Ok(())
}

#[test]
fn invalid_column_schemas_fail_before_creating_a_dataset() -> Result<(), Box<dyn Error>> {
  let directory = tempfile::tempdir()?;
  let namespace = attach(directory.path(), "data")?;
  let empty = namespace
    .create_table("invalid", &[])
    .err()
    .ok_or("empty schema was accepted")?;
  assert!(matches!(empty, LanceNamespaceError::EmptySchema));

  let duplicate = namespace
    .create_table(
      "invalid",
      &[
        LanceColumn {
          name: "id",
          data_type: LanceColumnType::BigInt,
        },
        LanceColumn {
          name: "ID",
          data_type: LanceColumnType::Varchar,
        },
      ],
    )
    .err()
    .ok_or("duplicate columns were accepted")?;
  assert!(matches!(
    duplicate,
    LanceNamespaceError::DuplicateColumn(ref name) if name == "ID"
  ));
  assert!(namespace.list_tables()?.is_empty());
  assert!(!directory.path().join("invalid.lance").exists());
  Ok(())
}
