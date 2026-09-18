//! The acceptance criterion the issue leads with, executed: 16,381 working-set
//! paths in a two-orientation lookup, against real stored edges.

use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{SharedBind, SharedBindList};

use crate::db::{setup_db, WeightRow};
use crate::seed::{
  owned_co_change, shared_co_change, working_set, CANDIDATE, EXPECTED_WEIGHT, WORKING_SET,
};

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// The whole point of issue #116: the same query, the same answer, half the
/// parameters — and it prepares, where the 32,767-parameter form does not.
#[test]
fn the_issue_query_binds_16_385_parameters_and_returns_the_right_weight() -> Outcome {
  let conn = setup_db()?;
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let query = shared_co_change(&candidate, &files);
  let (_, params) = query.to_sql_for(Dialect::Sqlite);
  assert_eq!(params.len(), WORKING_SET + 4);
  assert_eq!(params.len(), 16_385);

  let rows: Vec<WeightRow> = query.fetch_all(&conn)?;

  // 7 outgoing plus 11 incoming: a single-orientation query would report one
  // of them, and the wrong-relation and outside-the-set edges must not count.
  assert_eq!(
    rows,
    vec![WeightRow {
      weight: EXPECTED_WEIGHT
    }]
  );
  assert_eq!(EXPECTED_WEIGHT, 18);
  Ok(())
}

/// The form the issue reports as broken, on the same database: 32,767
/// parameters, which SQLite refuses to prepare.
#[test]
fn the_owned_form_exceeds_sqlites_variable_limit_on_the_same_input() -> Outcome {
  let conn = setup_db()?;
  let files = working_set();

  let (sql, params) = owned_co_change(CANDIDATE, &files).to_sql_for(Dialect::Sqlite);
  assert_eq!(params.len(), 32_767);

  let refused = conn.prepare(&sql);

  let message = match refused {
    Ok(_) => return Err("SQLite accepted 32_767 bound variables".into()),
    Err(error) => error.to_string(),
  };
  assert!(
    message.contains("variable number"),
    "unexpected refusal: {message}"
  );
  Ok(())
}

/// A handle used once costs exactly what the owned form costs, so reuse is
/// never a tax on the simple case.
#[test]
fn a_single_orientation_lookup_agrees_with_the_shared_one() -> Outcome {
  let conn = setup_db()?;
  let candidate = SharedBind::new(CANDIDATE);
  let files = SharedBindList::new(working_set());

  let shared: Vec<WeightRow> = shared_co_change(&candidate, &files).fetch_all(&conn)?;
  let small = working_set().into_iter().take(20).collect::<Vec<_>>();
  let owned: Vec<WeightRow> = owned_co_change(CANDIDATE, &small).fetch_all(&conn)?;

  // `file:r:7.rs` and `file:r:11.rs` are both inside the first 20 paths, so
  // the small owned form sees exactly the two edges the shared form sees.
  assert_eq!(shared, owned);
  assert_eq!(shared, vec![WeightRow { weight: 18 }]);
  Ok(())
}

/// Two handles over the same working set are two bindings, so the statement
/// grows — the independence rule, priced.
#[test]
fn two_distinct_list_handles_over_the_same_paths_bind_twice() {
  let candidate = SharedBind::new(CANDIDATE);
  let left = SharedBindList::new(working_set());
  let right = SharedBindList::new(working_set());

  let (_, shared) = shared_co_change(&candidate, &left).to_sql_for(Dialect::Sqlite);
  let (_, independent) =
    crate::seed::mixed_co_change(&candidate, &left, &right).to_sql_for(Dialect::Sqlite);

  assert_eq!(shared.len(), 16_385);
  assert_eq!(independent.len(), 16_385 + WORKING_SET);
}
