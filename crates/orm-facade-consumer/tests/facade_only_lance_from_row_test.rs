//! Real Lance rows decoded through the facade alone, including mixed features.

#[path = "fixtures/lance_from_row.rs"]
pub mod support;

mod errors {
  use std::error::Error;
  use toolu_orm::{
    connection::{DbConnectionBlocking, DbError},
    FromRow,
  };

  use crate::support::{session, Scalars};

  #[test]
  fn facade_derive_reports_missing_null_and_wrong_type() -> Result<(), Box<dyn Error>> {
    let (directory, conn) = session()?;
    for (sql, column, expected, reason) in [
      (
        "SELECT real, text, boolean, blob, nullable FROM scalars",
        "integer",
        "i64",
        "missing",
      ),
      (
        "SELECT NULL AS integer, real, text, boolean, blob, nullable FROM scalars",
        "integer",
        "i64",
        "NULL",
      ),
      (
        "SELECT text AS integer, real, text, boolean, blob, nullable FROM scalars",
        "integer",
        "i64",
        "incompatible",
      ),
      (
        "SELECT integer, real, text, boolean, blob FROM scalars",
        "nullable",
        "Option",
        "missing",
      ),
      (
        "SELECT integer, real, text, boolean, blob, integer AS nullable FROM scalars",
        "nullable",
        "Option",
        "incompatible",
      ),
    ] {
      let error = DbConnectionBlocking::query_map::<Scalars>(&conn, sql, vec![]).unwrap_err();
      assert!(
        matches!(error, DbError::RowMapping(ref message)
        if message.contains(column) && message.contains(expected) && message.contains(reason)),
        "{error}"
      );
    }
    drop(conn);
    directory.remove()?;
    Ok(())
  }

  fn normalize(mut raw: String) -> Result<String, &'static str> {
    if raw.is_empty() {
      Err("empty text rejected")
    } else {
      raw.make_ascii_uppercase();
      Ok(raw)
    }
  }

  #[derive(Debug, PartialEq, FromRow)]
  struct Converted {
    #[from_row(with = "normalize")]
    text: String,
  }

  #[test]
  fn facade_derive_runs_custom_conversion_and_preserves_errors() -> Result<(), Box<dyn Error>> {
    let (directory, conn) = session()?;
    let rows: Vec<Converted> =
      DbConnectionBlocking::query_map(&conn, "SELECT text FROM scalars WHERE boolean", vec![])?;
    assert_eq!(
      rows,
      vec![Converted {
        text: "O'REILLY — 東京".into()
      }]
    );
    let error = DbConnectionBlocking::query_map::<Converted>(
      &conn,
      "SELECT text FROM scalars WHERE NOT boolean",
      vec![],
    )
    .unwrap_err();
    assert!(
      matches!(error, DbError::RowMapping(ref message)
      if message.contains("text") && message.contains("empty text rejected")),
      "{error}"
    );
    drop(conn);
    directory.remove()?;
    Ok(())
  }
}

mod scalars {
  use std::error::Error;
  use toolu_orm::{connection::DbConnectionBlocking, core::row::FromRow};

  use crate::support::{session, Scalars};

  #[test]
  fn facade_derive_decodes_named_lance_scalars() -> Result<(), Box<dyn Error>> {
    let (directory, conn) = session()?;
    assert_eq!(
      Scalars::REQUIRED_COLUMNS,
      &["integer", "real", "text", "boolean", "blob", "nullable"]
    );
    let rows: Vec<Scalars> = DbConnectionBlocking::query_map(
      &conn,
      "SELECT nullable, blob, boolean, text, real, integer AS INTEGER FROM scalars ORDER BY integer",
      vec![],
    )?;
    assert_eq!(
      rows,
      vec![
        Scalars {
          integer: i64::MIN,
          real: -2.5,
          text: "O'Reilly — 東京".into(),
          boolean: true,
          blob: vec![0, 39, 255],
          nullable: None
        },
        Scalars {
          integer: i64::MAX,
          real: 0.0,
          text: "".into(),
          boolean: false,
          blob: vec![],
          nullable: Some("present".into())
        },
      ]
    );
    assert!(DbConnectionBlocking::query_map::<Scalars>(
      &conn,
      "SELECT * FROM scalars WHERE false",
      vec![]
    )?
    .is_empty());
    #[cfg(all(feature = "rusqlite", not(feature = "libsql")))]
    {
      // The same derived type also chooses its relational decoder in mixed builds.
      let sqlite = toolu_orm::core::rusqlite::Connection::open_in_memory()?;
      let mut statement = sqlite.prepare("SELECT 7, 2.5, 'sqlite', true, X'00FF', NULL")?;
      let mut cursor = statement.query([])?;
      let row = cursor.next()?.ok_or("missing SQLite row")?;
      let decoded = toolu_orm::core::row::from_rusqlite_row::<Scalars>(row)?;
      assert_eq!(
        decoded,
        Scalars {
          integer: 7,
          real: 2.5,
          text: "sqlite".into(),
          boolean: true,
          blob: vec![0, 255],
          nullable: None,
        }
      );
    }
    drop(conn);
    directory.remove()?;
    Ok(())
  }
}
