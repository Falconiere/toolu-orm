//! SQLite's table-valued `FROM` sources with bound arguments: `json_each` and
//! `pragma_table_info`, the two shapes the issue names.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

use crate::db::{setup_db, NameRow, ValueRow};
use crate::seed::SEED_VALUE;

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn json_each(json: &str) -> Result<TableRef, Box<dyn std::error::Error>> {
  Ok(TableRef::function("json_each", vec![Value::Text(json.to_owned())])?.with_alias("seeds"))
}

/// The argument is a bound parameter, so the JSON document never reaches the
/// SQL text.
#[test]
fn json_each_expands_a_bound_json_array_into_rows() -> Outcome {
  let conn = setup_db()?;
  let seeds = json_each(r#"["a","b","c"]"#)?;

  let rows: Vec<ValueRow> = SelectBuilder::from_table(&seeds)
    .column_as(&seeds.column(&SEED_VALUE), "value")
    .fetch_all(&conn)?;

  assert_eq!(
    rows
      .iter()
      .map(|row| row.value.as_str())
      .collect::<Vec<&str>>(),
    vec!["a", "b", "c"]
  );
  Ok(())
}

#[test]
fn an_empty_json_array_expands_to_no_rows() -> Outcome {
  let conn = setup_db()?;
  let seeds = json_each("[]")?;

  let rows: Vec<ValueRow> = SelectBuilder::from_table(&seeds)
    .column_as(&seeds.column(&SEED_VALUE), "value")
    .fetch_all(&conn)?;

  assert!(rows.is_empty());
  Ok(())
}

/// A function source joins like any other relation, and its argument binds
/// before the `ON` clause.
#[test]
fn a_function_source_joins_a_real_table() -> Outcome {
  let conn = setup_db()?;
  let seeds = json_each(r#"["c1","c3"]"#)?;

  let rows: Vec<crate::db::IdRow> = SelectBuilder::new("code_symbols")
    .column_as(&crate::seed::SYMBOL_ID, "id")
    .join(
      &seeds,
      crate::seed::SYMBOL_ID.equals(&seeds.column(&SEED_VALUE)),
    )
    .order_by(crate::seed::SYMBOL_ID.asc())
    .fetch_all(&conn)?;

  assert_eq!(crate::db::ids(&rows), vec!["c1", "c3"]);
  Ok(())
}

/// The `rebuild_copy.rs` shape: a pragma function reading a bound table name.
#[test]
fn pragma_table_info_reads_a_bound_table_name() -> Outcome {
  let conn = setup_db()?;
  let info = TableRef::function("pragma_table_info", vec![Value::Text("edges".to_owned())])?
    .with_alias("info");

  let rows: Vec<NameRow> = SelectBuilder::from_table(info)
    .columns_raw(&["name"])
    .fetch_all(&conn)?;

  assert_eq!(
    rows
      .iter()
      .map(|row| row.name.as_str())
      .collect::<Vec<&str>>(),
    vec!["src_kind", "src_id", "dst_kind", "dst_id"]
  );
  Ok(())
}

/// A hostile *alias* is quoted with its embedded quote doubled, so it names a
/// relation instead of ending the identifier — and the table it tried to drop
/// is still there.
#[test]
fn a_hostile_alias_is_quoted_not_executed() -> Outcome {
  let conn = setup_db()?;
  let seeds = TableRef::function("json_each", vec![Value::Text(r#"["a"]"#.to_owned())])?
    .with_alias(r#"x"; DROP TABLE edges; --"#);

  let rows: Vec<ValueRow> = SelectBuilder::from_table(&seeds)
    .column_as(&seeds.column(&SEED_VALUE), "value")
    .fetch_all(&conn)?;

  assert_eq!(rows.len(), 1);
  let edges: Vec<crate::db::NameRow> = SelectBuilder::from_table(TableRef::function(
    "pragma_table_info",
    vec![Value::Text("edges".to_owned())],
  )?)
  .columns_raw(&["name"])
  .fetch_all(&conn)?;
  assert_eq!(edges.len(), 4, "edges must still exist");
  Ok(())
}

/// The production walk, seeded straight from a bound JSON array instead of a
/// seeds table — the exact statement `edges_retrieval.rs` runs today.
#[test]
fn the_production_json_each_seeded_walk_terminates_on_the_cycle() -> Outcome {
  let conn = setup_db()?;

  let rows: Vec<crate::db::WalkRow> =
    crate::json_walk::json_seeded_walk(r#"["a"]"#, 2)?.fetch_all(&conn)?;

  assert_eq!(
    crate::db::walk_pairs(&rows),
    vec![("a", 0), ("b", 1), ("c", 2)]
  );
  Ok(())
}
