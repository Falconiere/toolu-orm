//! The reproduction from issue #116, rendered.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{SharedBind, SharedBindList};
use toolu_orm_core::value::Value;

use crate::seed::{owned_co_change, shared_co_change, working_set, CANDIDATE, WORKING_SET};

/// `2 * 16_381 + 5`, the count the issue reports — one past SQLite's default
/// `SQLITE_MAX_VARIABLE_NUMBER`.
const OWNED_PARAMS: usize = 2 * WORKING_SET + 5;

/// 16,381 files + one candidate + three separately bound fixed predicates.
const SHARED_PARAMS: usize = WORKING_SET + 4;

#[test]
fn the_owned_form_still_binds_every_input_twice() {
  let files = working_set();
  let (_, params) = owned_co_change(CANDIDATE, &files).to_sql_for(Dialect::Sqlite);

  assert_eq!(params.len(), OWNED_PARAMS);
  assert_eq!(params.len(), 32_767);
}

#[test]
fn the_shared_form_binds_each_input_once() {
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let (_, params) = shared_co_change(&candidate, &files).to_sql_for(Dialect::Sqlite);

  assert_eq!(params.len(), SHARED_PARAMS);
  assert_eq!(params.len(), 16_385);
}

#[test]
fn the_shared_parameters_are_the_fixed_predicates_then_the_candidate_then_the_files() {
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let (_, params) = shared_co_change(&candidate, &files).to_sql_for(Dialect::Sqlite);

  let mut expected = vec![
    Value::Text("co_changed".to_owned()),
    Value::Text("file".to_owned()),
    Value::Text("file".to_owned()),
    Value::Text(CANDIDATE.to_owned()),
  ];
  expected.extend(working_set());
  assert_eq!(params, expected);
}

#[test]
fn both_orientations_name_the_same_placeholders_on_sqlite() {
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let (sql, _) = shared_co_change(&candidate, &files).to_sql_for(Dialect::Sqlite);

  // The candidate is ?4 in both directions, and the working set occupies the
  // one contiguous run ?5 … ?16385 in both.
  assert!(sql.contains(r#"("edges"."src_id" = ?4 AND "edges"."dst_id" IN (?5, ?6,"#));
  assert!(sql.contains(r#"("edges"."dst_id" = ?4 AND "edges"."src_id" IN (?5, ?6,"#));
  assert_eq!(sql.matches("?16385, ?16385)").count(), 0);
  assert_eq!(sql.matches(", ?16385)").count(), 2);
  assert!(!sql.contains("?16386"));
}

#[test]
fn the_same_query_renders_dollar_placeholders_on_postgres() {
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let (sql, params) = shared_co_change(&candidate, &files).to_sql_for(Dialect::Postgres);

  assert_eq!(params.len(), 16_385);
  assert!(sql.contains(r#"("edges"."src_id" = $4 AND "edges"."dst_id" IN ($5, $6,"#));
  assert!(sql.contains(r#"("edges"."dst_id" = $4 AND "edges"."src_id" IN ($5, $6,"#));
  assert_eq!(sql.matches(", $16385)").count(), 2);
  assert!(!sql.contains("$16386"));
}

#[test]
fn rendering_the_same_builder_twice_gives_the_identical_statement() {
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());
  let query = shared_co_change(&candidate, &files);

  let first = query.to_sql_for(Dialect::Sqlite);
  let second = query.to_sql_for(Dialect::Sqlite);

  assert_eq!(first.0, second.0);
  assert_eq!(first.1, second.1);
}

#[test]
fn the_same_handles_bind_once_again_in_a_second_statement() {
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let (_, first) = shared_co_change(&candidate, &files).to_sql_for(Dialect::Sqlite);
  let (sql, second) = shared_co_change(&candidate, &files).to_sql_for(Dialect::Sqlite);

  // The ledger belongs to the render, not to the handle: the second statement
  // numbers from ?1 again and binds its own copy of every value.
  assert_eq!(first, second);
  assert!(sql.contains(r#""edges"."rel" = ?1"#));
}
