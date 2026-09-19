//! Regression coverage for issue #131: a numbered `?N` inside a raw fragment
//! is **fragment-relative**. `?1` means "the first value *I* bound", so it is
//! offset by the fragment's base exactly like a bare `?`, and the placeholder
//! can no longer disagree with the value it was written for.
//!
//! Before this fix the numbered branch of `number_raw_params` re-emitted the
//! author's literal index, so a fragment rendered anywhere but first compared
//! against another clause's value — it prepared fine and answered wrongly.
//!
//! Every assertion names its dialect: `to_sql_fragment` follows
//! `Dialect::CURRENT`, which is Postgres in the postgres lane and SQLite in the
//! default lane, and this binary compiles in both.

use toolu_orm_core::column::Text;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::expr::{Expr, Scalar};
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_core::value::Value;

fn repo_col() -> Column<Text> {
  Column::new("memories", "repo")
}

fn text(value: &str) -> Value {
  Value::from(value)
}

// ── The frame: ?N is offset by the fragment's base ───────────────────────────

#[test]
fn a_numbered_placeholder_takes_the_index_its_value_actually_lands_on() {
  let fragment = Expr::raw("x = ?1", vec![text("needle")]);

  let (sql, params) = fragment.to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, "x = ?3");
  assert_eq!(params, vec![text("needle")]);
}

#[test]
fn a_numbered_placeholder_at_base_one_is_unchanged() {
  let (sql, params) =
    Expr::raw("x = ?1", vec![text("needle")]).to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, "x = ?1");
  assert_eq!(params, vec![text("needle")]);
}

#[test]
fn one_number_used_twice_renders_one_index_and_binds_one_value() {
  let (sql, params) =
    Expr::raw("x = ?1 OR y = ?1", vec![text("needle")]).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, "x = ?3 OR y = ?3");
  assert_eq!(params, vec![text("needle")]);
}

#[test]
fn postgres_renders_the_same_frame_with_dollar_placeholders() {
  let (sql, params) =
    Expr::raw("x = ?1 OR y = ?1", vec![text("needle")]).to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, "x = $3 OR y = $3");
  assert_eq!(params, vec![text("needle")]);
}

#[test]
fn two_numbers_address_the_two_values_in_order() {
  let (sql, params) = Expr::raw(
    "x BETWEEN ?1 AND ?2",
    vec![Value::Integer(1), Value::Integer(9)],
  )
  .to_sql_fragment_for(4, Dialect::Sqlite);
  assert_eq!(sql, "x BETWEEN ?4 AND ?5");
  assert_eq!(params, vec![Value::Integer(1), Value::Integer(9)]);
}

#[test]
fn a_number_may_address_an_earlier_value_out_of_reading_order() {
  let (sql, params) = Expr::raw("x = ?2 AND y = ?1", vec![text("a"), text("b")])
    .to_sql_fragment_for(2, Dialect::Sqlite);
  assert_eq!(sql, "x = ?3 AND y = ?2");
  assert_eq!(params, vec![text("a"), text("b")]);
}

// ── The composition the issue reports ────────────────────────────────────────

#[test]
fn a_numbered_fragment_after_a_typed_predicate_compares_its_own_value() {
  let predicate = repo_col()
    .eq("comemory")
    .and(Expr::raw("x = ?1 OR y = ?1", vec![text("needle")]));

  let (sql, params) = predicate.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, r#"("memories"."repo" = ?1 AND x = ?2 OR y = ?2)"#);
  assert_eq!(params, vec![text("comemory"), text("needle")]);
}

// ── Bare and numbered in one fragment ────────────────────────────────────────

#[test]
fn a_bare_placeholder_takes_one_more_than_the_largest_number_assigned() {
  let fragment = Expr::raw(
    "(a = ?1 OR b = ?1) AND c = ?",
    vec![text("first"), text("second")],
  );

  let (sql, params) = fragment.to_sql_fragment_for(1, Dialect::Sqlite);
  assert_eq!(sql, "(a = ?1 OR b = ?1) AND c = ?2");
  assert_eq!(params, vec![text("first"), text("second")]);
}

#[test]
fn the_mixed_fragment_shifts_whole_at_an_offset() {
  let fragment = Expr::raw(
    "(a = ?1 OR b = ?1) AND c = ?",
    vec![text("first"), text("second")],
  );

  let (sql, params) = fragment.to_sql_fragment_for(4, Dialect::Sqlite);
  assert_eq!(sql, "(a = ?4 OR b = ?4) AND c = ?5");
  assert_eq!(params, vec![text("first"), text("second")]);
}

#[test]
fn a_number_after_a_bare_placeholder_can_name_it() {
  let (sql, params) =
    Expr::raw("a = ? AND b = ?1", vec![text("only")]).to_sql_fragment_for(2, Dialect::Sqlite);
  assert_eq!(sql, "a = ?2 AND b = ?2");
  assert_eq!(params, vec![text("only")]);
}

// ── A bare-only fragment is untouched by this change ─────────────────────────

#[test]
fn a_bare_only_fragment_renders_exactly_as_it_did_before() {
  let fragment = Expr::raw("a = ? AND b = ?", vec![Value::Integer(1), text("x")]);

  let (sqlite, params) = fragment.to_sql_fragment_for(4, Dialect::Sqlite);
  assert_eq!(sqlite, "a = ?4 AND b = ?5");
  assert_eq!(params, vec![Value::Integer(1), text("x")]);

  let (postgres, _) = fragment.to_sql_fragment_for(4, Dialect::Postgres);
  assert_eq!(postgres, "a = $4 AND b = $5");
}

// ── Scalar::raw follows the identical rule ───────────────────────────────────

#[test]
fn a_scalar_fragment_is_numbered_from_its_own_base() {
  let scalar = Scalar::raw("substr(?1, 1, ?2)", vec![text("abcdef"), Value::Integer(3)]);

  let (sql, params) = scalar.to_sql_fragment_for(2, Dialect::Sqlite);
  assert_eq!(sql, "substr(?2, 1, ?3)");
  assert_eq!(params, vec![text("abcdef"), Value::Integer(3)]);
}

#[test]
fn a_scalar_fragment_reuses_one_value_across_two_occurrences() {
  let scalar = Scalar::raw("max(?1, abs(?1))", vec![Value::Integer(7)]);

  let (sql, params) = scalar.to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(sql, "max($3, abs($3))");
  assert_eq!(params, vec![Value::Integer(7)]);
}

// ── Indices that address no value of the fragment ────────────────────────────

#[test]
fn index_zero_is_not_a_placeholder_and_stays_verbatim() {
  let fragment = Expr::raw("x = ?0", vec![text("needle")]);

  let (sqlite, params) = fragment.to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sqlite, "x = ?0");
  assert_eq!(params, vec![text("needle")]);

  // Postgres gets the author's text too: `?0` addresses nothing to rewrite,
  // and both engines refuse to prepare it — the loudest outcome available to
  // an infallible constructor.
  let (postgres, _) = fragment.to_sql_fragment_for(3, Dialect::Postgres);
  assert_eq!(postgres, "x = ?0");
}

#[test]
fn an_index_past_the_supplied_values_is_shifted_like_any_other() {
  // An over-count is an authoring bug — the same one a fragment with more bare
  // `?` than values has. It stays in the fragment's own frame rather than
  // silently addressing an absolute index in somebody else's.
  let (sql, params) =
    Expr::raw("x = ?3", vec![text("a"), text("b")]).to_sql_fragment_for(5, Dialect::Sqlite);
  assert_eq!(sql, "x = ?7");
  assert_eq!(params, vec![text("a"), text("b")]);
}

#[test]
fn a_digit_run_too_large_for_an_index_stays_verbatim_and_never_panics() {
  let huge = "x = ?99999999999999999999999999999999999999";
  let (sql, params) = Expr::raw(huge, vec![text("needle")]).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, huge);
  assert_eq!(params, vec![text("needle")]);
}

#[test]
fn an_index_whose_shift_would_overflow_stays_verbatim() {
  let near_max = format!("x = ?{}", usize::MAX);
  let (sql, params) =
    Expr::raw(near_max.clone(), vec![text("needle")]).to_sql_fragment_for(3, Dialect::Sqlite);
  assert_eq!(sql, near_max);
  assert_eq!(params, vec![text("needle")]);
}
