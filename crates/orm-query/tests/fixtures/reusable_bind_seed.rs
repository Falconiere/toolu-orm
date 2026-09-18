//! The `edges` table the reusable-binding scenarios read, the working set the
//! issue reports on, and the two shapes of its co-change query.
//!
//! The working set is 16,381 file ids, which is what makes the difference
//! observable: bound once per occurrence it needs 32,767 parameters, one past
//! SQLite's default `SQLITE_MAX_VARIABLE_NUMBER` of 32,766; bound once per
//! handle it needs 16,385.

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::expr::{SharedBind, SharedBindList};
use toolu_orm_core::query_column::{Column, CommonOps, SharedOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::SelectBuilder;

pub const REL: Column<Text> = Column::new("edges", "rel");
pub const SRC_KIND: Column<Text> = Column::new("edges", "src_kind");
pub const SRC_ID: Column<Text> = Column::new("edges", "src_id");
pub const DST_KIND: Column<Text> = Column::new("edges", "dst_kind");
pub const DST_ID: Column<Text> = Column::new("edges", "dst_id");
pub const WEIGHT: Column<Integer> = Column::new("edges", "weight");

/// The `INSERT … SELECT` target: one row per path the source projected.
pub const TOUCHED_ID: Column<Text> = Column::new("touched", "id");
pub const TOUCHED_NOTE: Column<Text> = Column::new("touched", "note");

/// How many working-set paths the issue reports on.
pub const WORKING_SET: usize = 16_381;

/// The file the co-change weight is asked about.
pub const CANDIDATE: &str = "file:r:candidate.rs";

/// The two working-set entries the seeded edges actually touch, and the
/// weights they carry. They sum to 18, the answer every driver must give.
pub const OUTGOING: (&str, i64) = ("file:r:7.rs", 7);
pub const INCOMING: (&str, i64) = ("file:r:11.rs", 11);

/// The issue's working set: 16,381 distinct file ids.
pub fn working_set() -> Vec<Value> {
  (0..WORKING_SET)
    .map(|n| Value::Text(format!("file:r:{n}.rs")))
    .collect()
}

/// The aggregated weight, cast so both engines report a 64-bit integer:
/// Postgres widens `SUM(bigint)` to `numeric`, SQLite keeps it an integer.
/// The projection binds nothing, so the filters own every placeholder.
pub const WEIGHT_SUM: &str = r#"CAST(COALESCE(SUM("edges"."weight"), 0) AS BIGINT)"#;

/// The three fixed predicates every form of the query carries.
fn co_change_base() -> SelectBuilder {
  SelectBuilder::new("edges")
    .column_expr(WEIGHT_SUM, "weight")
    .filter(REL.eq("co_changed"))
    .filter(SRC_KIND.eq("file"))
    .filter(DST_KIND.eq("file"))
}

/// The issue's query with reusable bindings: the candidate and the working set
/// are each bound once and referenced from both orientations.
pub fn shared_co_change(candidate: &SharedBind, files: &SharedBindList) -> SelectBuilder {
  co_change_base().filter(
    SRC_ID
      .eq_shared(candidate)
      .and(DST_ID.in_shared(files))
      .or(DST_ID.eq_shared(candidate).and(SRC_ID.in_shared(files))),
  )
}

/// The same query with a *different* list handle per orientation, which is how
/// two handles over equal values stay independent: each binds its own run.
pub fn mixed_co_change(
  candidate: &SharedBind,
  outgoing: &SharedBindList,
  incoming: &SharedBindList,
) -> SelectBuilder {
  co_change_base().filter(
    SRC_ID
      .eq_shared(candidate)
      .and(DST_ID.in_shared(outgoing))
      .or(DST_ID.eq_shared(candidate).and(SRC_ID.in_shared(incoming))),
  )
}

/// The same query written with `eq` / `in_list`, which binds every input twice
/// — the form the issue reports as unusable past 16,380 paths.
pub fn owned_co_change(candidate: &str, files: &[Value]) -> SelectBuilder {
  co_change_base().filter(
    SRC_ID
      .eq(candidate)
      .and(DST_ID.in_list(files))
      .or(DST_ID.eq(candidate).and(SRC_ID.in_list(files))),
  )
}

/// `CREATE TABLE "edges"` for SQLite and libsql.
pub const SQLITE_DDL: &str = r#"CREATE TABLE "edges" (
  "rel" TEXT NOT NULL,
  "src_kind" TEXT NOT NULL,
  "src_id" TEXT NOT NULL,
  "dst_kind" TEXT NOT NULL,
  "dst_id" TEXT NOT NULL,
  "weight" INTEGER NOT NULL
)"#;

/// `CREATE TABLE "touched"` — the same text on both engines.
pub const TOUCHED_DDL: &str =
  r#"CREATE TABLE "touched" ("id" TEXT PRIMARY KEY, "note" TEXT NOT NULL)"#;

/// `CREATE TABLE "edges"` for Postgres.
pub const POSTGRES_DDL: &str = r#"CREATE TABLE "edges" (
  "rel" TEXT NOT NULL,
  "src_kind" TEXT NOT NULL,
  "src_id" TEXT NOT NULL,
  "dst_kind" TEXT NOT NULL,
  "dst_id" TEXT NOT NULL,
  "weight" BIGINT NOT NULL
)"#;

/// The co-change edges: one in each direction, so a single-orientation query
/// would report 7 or 11 rather than the correct 18.
pub const EDGES_SEED: &[(&str, &str, &str, &str, &str, i64)] = &[
  (
    "co_changed",
    "file",
    CANDIDATE,
    "file",
    OUTGOING.0,
    OUTGOING.1,
  ),
  (
    "co_changed",
    "file",
    INCOMING.0,
    "file",
    CANDIDATE,
    INCOMING.1,
  ),
  // Same pair, wrong relation: proves the fixed predicates still filter.
  ("imports", "file", CANDIDATE, "file", OUTGOING.0, 100),
  // Right relation, but the other end is outside the working set.
  (
    "co_changed",
    "file",
    CANDIDATE,
    "file",
    "file:r:outside.rs",
    50,
  ),
];

/// The weights the two co-change edges carry.
pub const EXPECTED_WEIGHT: i64 = OUTGOING.1 + INCOMING.1;
