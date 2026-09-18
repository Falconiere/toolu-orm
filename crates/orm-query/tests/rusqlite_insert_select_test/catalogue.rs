//! A database-qualified `SelectBuilder` reading the *attached* store's
//! catalogue — the presence check comemory's rebuild runs before copying a
//! table an older store may not have.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::column::Text;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::row::FromRow;
use toolu_orm_query::select::SelectBuilder;

use super::db::TestResult;
use super::support::{Attached, OLD};

const OBJECT_TYPE: Column<Text> = Column::new("sqlite_master", "type");
const OBJECT_NAME: Column<Text> = Column::new("sqlite_master", "name");

#[derive(Debug, PartialEq)]
struct ObjectName(String);

impl FromRow for ObjectName {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["name"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    Ok(Self(
      row
        .get(0)
        .map_err(|e| DbCoreError::RowMapping(e.to_string()))?,
    ))
  }
}

/// Every table name in `database`, ordered.
fn tables_in(database: &str) -> SelectBuilder {
  SelectBuilder::from_table(TableRef::new("sqlite_master").in_database(database))
    .columns_raw(&["name"])
    .filter(OBJECT_TYPE.eq("table"))
    .order_by(OBJECT_NAME.asc())
}

#[test]
fn the_qualifier_selects_which_databases_catalogue_is_read() -> TestResult {
  let fixture = Attached::open("catalogue")?;
  let attached = fixture.attach()?;

  let old: Vec<ObjectName> = tables_in(OLD).fetch_all(&fixture.conn)?;
  let main: Vec<ObjectName> = tables_in("main").fetch_all(&fixture.conn)?;
  drop(attached);

  assert!(
    old.contains(&ObjectName("legacy_notes".to_owned())),
    "the older store's own table: {old:?}"
  );
  assert!(
    !old.contains(&ObjectName("sync_state".to_owned())),
    "sync_state exists only in the target: {old:?}"
  );
  assert!(
    main.contains(&ObjectName("sync_state".to_owned())),
    "{main:?}"
  );
  assert!(
    !main.contains(&ObjectName("legacy_notes".to_owned())),
    "{main:?}"
  );
  assert_ne!(old, main, "one builder, two databases, two answers");
  Ok(())
}
