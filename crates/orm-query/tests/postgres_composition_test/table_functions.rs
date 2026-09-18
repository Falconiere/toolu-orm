//! A table-valued `FROM` source with bound arguments on Postgres.
//!
//! The *mechanism* — a function call in a `FROM` slot whose arguments are bound
//! parameters — is portable; the function names are not, exactly as with
//! `Scalar::func`. `regexp_split_to_table(text, text)` is the Postgres stand-in
//! for the SQLite suites' `json_each`: it takes two bound arguments and its
//! parameter types infer unambiguously, which `generate_series($1, $2)` does
//! not.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{setup_db, PartRow};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn split(text: &str, pattern: &str) -> Result<TableRef, Box<dyn std::error::Error>> {
  Ok(
    TableRef::function(
      "regexp_split_to_table",
      vec![
        Value::Text(text.to_owned()),
        Value::Text(pattern.to_owned()),
      ],
    )?
    .with_alias("parts"),
  )
}

#[test]
fn the_source_renders_both_arguments_as_bound_parameters() -> Outcome {
  let parts = split("a,b,c", ",")?;

  let (sql, params) = SelectBuilder::from_table(&parts)
    .columns_raw(&["parts"])
    .to_sql_for(Dialect::Postgres);

  assert_eq!(
    sql,
    r#"SELECT "parts" FROM regexp_split_to_table($1, $2) AS "parts""#
  );
  assert_eq!(params.len(), 2);
  Ok(())
}

#[tokio::test]
async fn a_table_valued_source_expands_its_bound_arguments_into_rows() -> Outcome {
  let client = setup_db("composition_tvf_1").await?;
  let parts = split("a,b,c", ",")?;

  let rows: Vec<PartRow> = SelectBuilder::from_table(&parts)
    .columns_raw(&["parts"])
    .fetch_all(&client)
    .await?;

  assert_eq!(
    rows
      .iter()
      .map(|row| row.parts.as_str())
      .collect::<Vec<&str>>(),
    vec!["a", "b", "c"]
  );
  Ok(())
}

/// A separator that never occurs yields the whole input as one row, which
/// proves the second argument is really being bound and used.
#[tokio::test]
async fn a_separator_that_does_not_occur_yields_one_row() -> Outcome {
  let client = setup_db("composition_tvf_2").await?;
  let parts = split("a,b,c", ";")?;

  let rows: Vec<PartRow> = SelectBuilder::from_table(&parts)
    .columns_raw(&["parts"])
    .fetch_all(&client)
    .await?;

  assert_eq!(
    rows
      .iter()
      .map(|row| row.parts.as_str())
      .collect::<Vec<&str>>(),
    vec!["a,b,c"]
  );
  Ok(())
}
