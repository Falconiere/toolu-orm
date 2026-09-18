//! Table aliases, qualified projections and expression `ON` clauses executed
//! against an in-memory rusqlite database (rusqlite-only lane, synchronous).
//!
//! The same five scenarios run on libsql and Postgres; the rendered SQL is
//! pinned separately in `join_alias_sql_test`.

#[path = "fixtures/rusqlite_join_db.rs"]
pub mod db;
#[path = "fixtures/join_tables.rs"]
pub mod tables;

use toolu_orm_core::alias::TableRef;
use toolu_orm_query::select::SelectBuilder;

use tables::{
  ambiguous_query, error_text, ids, labels, left_join_on_query, left_join_where_query,
  on_and_where_query, qualified_ids_query, self_join_query, two_joins_query, Ids, Labelled, Labels,
  Pair, S_ID, S_KIND,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn self_join_under_two_aliases_returns_expected_pairs() -> TestResult {
  let conn = db::setup_db()?;
  let pairs: Vec<Pair> = self_join_query().fetch_all(&conn)?;

  // Owner o1 holds s1(10), s2(20), s4(40); o2 holds only s3, so it pairs with
  // nothing. Ordering is by old.id then newer.id.
  assert_eq!(
    pairs,
    vec![
      Pair {
        old_id: "s1".to_owned(),
        newer_id: "s2".to_owned()
      },
      Pair {
        old_id: "s1".to_owned(),
        newer_id: "s4".to_owned()
      },
      Pair {
        old_id: "s2".to_owned(),
        newer_id: "s4".to_owned()
      },
    ]
  );
  Ok(())
}

#[test]
fn same_table_joined_twice_projects_both_labels() -> TestResult {
  let conn = db::setup_db()?;
  let rows: Vec<Labels> = two_joins_query().fetch_all(&conn)?;

  // Two LEFT JOINs to `code_feedback`, each binding its own label in its own ON
  // clause: the second join's placeholder continues after the first's.
  assert_eq!(
    rows,
    vec![
      Labels {
        s_id: "s1".to_owned(),
        up_label: Some("up".to_owned()),
        down_label: None
      },
      Labels {
        s_id: "s2".to_owned(),
        up_label: None,
        down_label: Some("down".to_owned())
      },
      Labels {
        s_id: "s3".to_owned(),
        up_label: Some("up".to_owned()),
        down_label: None
      },
      Labels {
        s_id: "s4".to_owned(),
        up_label: None,
        down_label: None
      },
    ]
  );
  Ok(())
}

#[test]
fn unqualified_projection_is_ambiguous_but_qualified_one_decodes() -> TestResult {
  let conn = db::setup_db()?;

  // Both joined tables have an `id`, so the bare name has no referent.
  let rejected = ambiguous_query().fetch_all::<Labelled>(&conn);
  let Err(err) = rejected else {
    return Err("an unqualified `id` across two joined tables must be rejected".into());
  };
  let message = error_text(&err);
  assert!(
    message.contains("ambiguous"),
    "expected an ambiguity error, got: {message}"
  );

  let rows: Vec<Ids> = qualified_ids_query().fetch_all(&conn)?;
  assert_eq!(
    rows.len(),
    4,
    "every symbol keeps its row through the LEFT JOIN"
  );
  let s4 = rows
    .iter()
    .find(|r| r.s_id == "s4")
    .ok_or("s4 missing from the qualified projection")?;
  assert_eq!(s4.f_id.as_deref(), Some("f4"));
  for row in &rows {
    assert_ne!(
      Some(row.s_id.as_str()),
      row.f_id.as_deref(),
      "the two projected ids come from different tables"
    );
  }
  Ok(())
}

#[test]
fn left_join_on_predicate_keeps_unmatched_rows_where_drops_them() -> TestResult {
  let conn = db::setup_db()?;

  let kept: Vec<Labelled> = left_join_on_query(1).fetch_all(&conn)?;
  assert_eq!(ids(&kept), vec!["s1", "s2", "s3", "s4"]);
  assert_eq!(
    labels(&kept),
    vec![Some("up"), Some("down"), Some("up"), None],
    "s4's only feedback row is live = 0, so it comes back unmatched"
  );

  let dropped: Vec<Labelled> = left_join_where_query(1).fetch_all(&conn)?;
  assert_eq!(
    ids(&dropped),
    vec!["s1", "s2", "s3"],
    "the same predicate in WHERE discards the unmatched row"
  );
  assert_eq!(kept.len(), 4);
  assert_eq!(dropped.len(), 3);
  Ok(())
}

#[test]
fn on_and_where_params_select_the_expected_rows() -> TestResult {
  let conn = db::setup_db()?;

  let live_rows: Vec<Labelled> = on_and_where_query(1, "note").fetch_all(&conn)?;
  assert_eq!(ids(&live_rows), vec!["s1", "s2", "s4"]);
  assert_eq!(labels(&live_rows), vec![Some("up"), Some("down"), None]);

  // Only the ON parameter changes: the same three rows come back, now matched
  // against the stale feedback instead. A swapped binding could not do this.
  let stale_rows: Vec<Labelled> = on_and_where_query(0, "note").fetch_all(&conn)?;
  assert_eq!(ids(&stale_rows), vec!["s1", "s2", "s4"]);
  assert_eq!(labels(&stale_rows), vec![None, None, Some("stale")]);

  // Only the WHERE parameter changes.
  let task_rows: Vec<Labelled> = on_and_where_query(1, "task").fetch_all(&conn)?;
  assert_eq!(ids(&task_rows), vec!["s3"]);

  assert_eq!(on_and_where_query(1, "note").count(&conn)?, 3);
  assert!(on_and_where_query(1, "note").exists(&conn)?);
  assert_eq!(on_and_where_query(1, "absent").count(&conn)?, 0);
  assert!(!on_and_where_query(1, "absent").exists(&conn)?);
  Ok(())
}

/// The identifier quoting, proven by execution rather than by string equality.
///
/// A double quote inside an alias is doubled, which is the only escape SQLite
/// and Postgres have; there is no backslash escape, so once the interior quotes
/// are doubled the `;`, the statement that follows it and the `--` are ordinary
/// characters of one identifier. `rusqlite::Connection::prepare` also refuses
/// multi-statement text outright.
#[test]
fn an_injection_shaped_alias_is_quoted_not_executed() -> TestResult {
  let conn = db::setup_db()?;
  let hostile = TableRef::aliased("code_symbols", r#"x"; DROP TABLE code_symbols; --"#);

  let query = SelectBuilder::from_table(&hostile)
    .column_as(&hostile.column(&S_ID), "s_id")
    .column_as(&hostile.column(&S_KIND), "label")
    .order_by(hostile.column(&S_ID).asc());

  // The alias is one identifier: every interior quote is doubled, so the `;`
  // and everything after it stay inside it.
  let (sql, params) = query.to_sql();
  assert_eq!(
    sql,
    concat!(
      r#"SELECT "x""; DROP TABLE code_symbols; --"."id" AS "s_id", "#,
      r#""x""; DROP TABLE code_symbols; --"."kind" AS "label""#,
      r#" FROM "code_symbols" AS "x""; DROP TABLE code_symbols; --""#,
      r#" ORDER BY "x""; DROP TABLE code_symbols; --"."id" ASC"#
    )
  );
  assert!(params.is_empty());

  let rows: Vec<Labelled> = query.fetch_all(&conn)?;
  assert_eq!(ids(&rows), vec!["s1", "s2", "s3", "s4"]);
  assert_eq!(
    labels(&rows),
    vec![Some("note"), Some("note"), Some("task"), Some("note")]
  );
  assert_eq!(
    SelectBuilder::new("code_symbols").count(&conn)?,
    4,
    "the table named inside the alias still has all of its rows"
  );
  Ok(())
}
