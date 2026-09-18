//! `DELETE … WHERE id IN (SELECT …)`: the owner-id set stays in the database.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::value::Value;
use toolu_orm_query::delete::DeleteBuilder;

use crate::db::{setup_db, VecRow};
use crate::queries::{ids_of_repo_path, remaining_vec_rows};
use crate::seed::VEC_SYMBOL_ID;

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn prune(repo: &str, path: &str) -> DeleteBuilder {
  DeleteBuilder::new("code_vec")
    .filter(Scalar::col(&VEC_SYMBOL_ID).in_subquery(ids_of_repo_path(repo, path)))
}

/// One statement removes exactly the rows whose symbol matches, and the other
/// repo's rows survive.
#[test]
fn the_delete_removes_only_the_matching_rows() -> Outcome {
  let conn = setup_db()?;

  prune("r1", "src/a.rs").execute(&conn)?;
  let rows: Vec<VecRow> = remaining_vec_rows().fetch_all(&conn)?;

  assert_eq!(
    rows
      .iter()
      .map(|row| (row.symbol_id.as_str(), row.note.as_str()))
      .collect::<Vec<(&str, &str)>>(),
    vec![("c2", "v2"), ("c3", "v3")]
  );
  Ok(())
}

/// The point of the feature: the parameter vector carries the repo and the
/// path, and *not* the id set — which therefore never crossed into Rust.
#[test]
fn the_statement_binds_only_the_subquerys_own_values() {
  let (sql, params) = prune("r1", "src/a.rs").to_sql_for(Dialect::Sqlite);

  assert_eq!(
    params,
    vec![
      Value::Text("r1".to_owned()),
      Value::Text("src/a.rs".to_owned())
    ]
  );
  assert!(
    sql.contains(r#"IN (SELECT "code_symbols"."id""#),
    "got: {sql}"
  );
}

#[test]
fn a_subquery_matching_nothing_deletes_nothing() -> Outcome {
  let conn = setup_db()?;

  prune("r9", "nowhere.rs").execute(&conn)?;
  let rows: Vec<VecRow> = remaining_vec_rows().fetch_all(&conn)?;

  assert_eq!(rows.len(), 3);
  Ok(())
}

/// A compound subquery works in the same slot, so a prune can span branches.
#[test]
fn the_predicate_accepts_a_compound_subquery() -> Outcome {
  let conn = setup_db()?;

  DeleteBuilder::new("code_vec")
    .filter(
      Scalar::col(&VEC_SYMBOL_ID)
        .in_subquery(ids_of_repo_path("r1", "src/a.rs").union(ids_of_repo_path("r2", "src/a.rs"))),
    )
    .execute(&conn)?;
  let rows: Vec<VecRow> = remaining_vec_rows().fetch_all(&conn)?;

  assert_eq!(
    rows
      .iter()
      .map(|row| row.symbol_id.as_str())
      .collect::<Vec<&str>>(),
    vec!["c2"]
  );
  Ok(())
}
