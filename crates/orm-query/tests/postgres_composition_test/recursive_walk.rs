//! The acceptance criterion the issue leads with: a recursive walk that
//! terminates on a cycle and honours a depth limit, against a real graph.

use crate::db::{setup_db, walk_pairs, WalkRow};
use crate::queries::walk_rows;

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// `a → b → c → a` is a cycle: without `UNION` collapsing a node already
/// reached, this query would never return. It does, and the depth limit — not
/// the cycle — decides where it stops.
#[tokio::test]
async fn the_walk_terminates_on_a_cycle_and_stops_at_the_depth_limit() -> Outcome {
  let conn = setup_db("composition_walk_1").await?;

  let rows: Vec<WalkRow> = walk_rows("b1", 2).fetch_all(&conn).await?;

  assert_eq!(walk_pairs(&rows), vec![("a", 0), ("b", 1), ("c", 2)]);
  Ok(())
}

/// One more hop reaches `d`, which proves the previous result was bounded by
/// the limit rather than by the graph running out.
#[tokio::test]
async fn a_larger_limit_reaches_the_node_one_hop_further() -> Outcome {
  let conn = setup_db("composition_walk_2").await?;

  let rows: Vec<WalkRow> = walk_rows("b1", 3).fetch_all(&conn).await?;

  assert_eq!(
    walk_pairs(&rows),
    vec![("a", 0), ("b", 1), ("c", 2), ("d", 3)]
  );
  Ok(())
}

#[tokio::test]
async fn a_zero_limit_returns_only_the_seed() -> Outcome {
  let conn = setup_db("composition_walk_3").await?;

  let rows: Vec<WalkRow> = walk_rows("b1", 0).fetch_all(&conn).await?;

  assert_eq!(walk_pairs(&rows), vec![("a", 0)]);
  Ok(())
}

/// `d` has no outgoing edge, so a walk seeded there stops immediately however
/// generous the limit.
#[tokio::test]
async fn a_seed_with_no_outgoing_edge_returns_only_itself() -> Outcome {
  let conn = setup_db("composition_walk_4").await?;

  let rows: Vec<WalkRow> = walk_rows("b2", 9).fetch_all(&conn).await?;

  assert_eq!(walk_pairs(&rows), vec![("d", 0)]);
  Ok(())
}

/// `MIN(depth)` is what makes the answer the *shortest* path: `a` is reachable
/// again at depth 3 through the cycle, and still reports 0.
#[tokio::test]
async fn a_node_reachable_by_two_paths_reports_its_shallowest_depth() -> Outcome {
  let conn = setup_db("composition_walk_5").await?;

  let rows: Vec<WalkRow> = walk_rows("b1", 5).fetch_all(&conn).await?;

  assert_eq!(
    walk_pairs(&rows),
    vec![("a", 0), ("b", 1), ("c", 2), ("d", 3)]
  );
  Ok(())
}

/// The counted form of a CTE statement wraps a derived table and still reads
/// the CTE, so it reports the number of groups the walk returns.
#[tokio::test]
async fn counting_the_walk_reports_the_number_of_reached_nodes() -> Outcome {
  let conn = setup_db("composition_walk_6").await?;

  assert_eq!(walk_rows("b1", 2).count(&conn).await?, 3);
  assert_eq!(walk_rows("b1", 3).count(&conn).await?, 4);
  assert!(walk_rows("b1", 0).exists(&conn).await?);
  Ok(())
}
