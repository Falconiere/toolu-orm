//! `toolu_orm_core::row::from_libsql_row` against real libsql rows.
//!
//! The helper exists so that no crate outside orm-core has to guess which
//! `FromRow` method Cargo's feature unification produced (issue #124). This
//! binary is gated on nothing but `libsql`, so it compiles in whatever shape the
//! lane gives orm-core and proves the helper picks the right method there. The
//! postgres lane runs it on the two-driver shape, where `FromRow` exposes
//! `from_pg_row` + `from_libsql_row` and no `from_row` at all.
#![cfg(feature = "libsql")]

use toolu_orm_core::row::from_libsql_row;
use toolu_orm_macros::FromRow;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(FromRow, Debug, PartialEq)]
struct Person {
  id: String,
  age: Option<i64>,
}

/// One NULL age and one present age, so decoding has to distinguish them.
const SETUP: &str = "CREATE TABLE people (id TEXT PRIMARY KEY, age INTEGER); \
   INSERT INTO people (id, age) VALUES ('p1', NULL); \
   INSERT INTO people (id, age) VALUES ('p2', 30);";

async fn people_conn() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn.execute_batch(SETUP).await?;
  Ok(conn)
}

#[tokio::test]
async fn from_libsql_row_decodes_real_rows_with_null_as_none() -> TestResult {
  let conn = people_conn().await?;
  let mut rows = conn
    .query("SELECT id, age FROM people ORDER BY id", ())
    .await?;

  let mut people = Vec::new();
  while let Some(row) = rows.next().await? {
    people.push(from_libsql_row::<Person>(&row)?);
  }

  assert_eq!(
    people,
    vec![
      Person {
        id: "p1".into(),
        age: None
      },
      Person {
        id: "p2".into(),
        age: Some(30)
      },
    ]
  );
  Ok(())
}

/// The helper forwards the decoder's own verdict and adds no validation of its
/// own. libsql reads an out-of-range index as SQL NULL, so a projection without
/// `age` decodes to `None` rather than failing -- unlike Postgres, where the
/// same narrowed projection is a `RowMapping` error
/// (`from_row_derive_live_test::derive_fewer_columns_than_required_is_row_mapping`),
/// and unlike rusqlite, whose twin of this test asserts the error. Pinning the
/// difference here keeps it from being mistaken for a helper bug.
#[tokio::test]
async fn from_libsql_row_absent_column_decodes_as_none() -> TestResult {
  let conn = people_conn().await?;
  let mut rows = conn.query("SELECT id FROM people ORDER BY id", ()).await?;
  let Some(row) = rows.next().await? else {
    return Err("expected at least one row".into());
  };

  assert_eq!(
    from_libsql_row::<Person>(&row)?,
    Person {
      id: "p1".into(),
      age: None
    }
  );
  Ok(())
}
