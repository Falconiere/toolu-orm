//! The composed builders every driver suite executes.
//!
//! Dialect-neutral by construction: the walk seeds from a real `walk_seeds`
//! row rather than from `json_each`, which is SQLite-only, so one builder shape
//! runs on rusqlite, libsql and Postgres alike. The SQLite-only table-valued
//! sources live in their own suites.
//!
//! `CAST(0 AS BIGINT)` rather than a bare `0`: Postgres types a bare literal as
//! `integer`, which would make the whole `depth` column `int4` and `MIN(depth)`
//! undecodable into an `i64`. SQLite reads the cast as plain INTEGER.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::expr::{Expr, Scalar};
use toolu_orm_core::query_column::{CommonOps, NumericOps};
use toolu_orm_query::select::{Cte, SelectBuilder};

use super::seed::{
  EDGE_DST_ID, EDGE_DST_KIND, EDGE_SRC_ID, EDGE_SRC_KIND, ITEM_ID, ITEM_OWNER_ID, OWNER_ID,
  SEED_BATCH, SEED_ID, SEED_KIND, SYMBOL_ID, SYMBOL_PATH, SYMBOL_REPO, VEC_SYMBOL_ID, WALK_DEPTH,
  WALK_ID, WALK_KIND,
};

/// The walk's starting rows, read from a real table under a bound batch — the
/// dialect-neutral anchor every driver shares.
pub fn seeded_anchor(batch: &str) -> SelectBuilder {
  let seeds = TableRef::aliased("walk_seeds", "w");
  SelectBuilder::from_table(&seeds)
    .column_as(&seeds.column(&SEED_KIND), "kind")
    .column_as(&seeds.column(&SEED_ID), "id")
    .column_scalar(Scalar::sql("CAST(0 AS BIGINT)"), "depth")
    .filter(seeds.column(&SEED_BATCH).eq(batch))
}

/// One hop: every edge out of a node already in `walk`, one level deeper,
/// while the depth is under the bound.
pub fn walk_step(max_depth: i64) -> SelectBuilder {
  let edges = TableRef::aliased("edges", "e");
  let walk = TableRef::aliased("walk", "s");

  SelectBuilder::from_table(&edges)
    .column_as(&edges.column(&EDGE_DST_KIND), "kind")
    .column_as(&edges.column(&EDGE_DST_ID), "id")
    .column_scalar(
      Scalar::sql(walk.column(&WALK_DEPTH).qualified()) + Scalar::sql("1"),
      "depth",
    )
    .join(
      &walk,
      edges
        .column(&EDGE_SRC_KIND)
        .equals(&walk.column(&WALK_KIND))
        .and(edges.column(&EDGE_SRC_ID).equals(&walk.column(&WALK_ID))),
    )
    .filter(walk.column(&WALK_DEPTH).lt(max_depth))
}

/// The recursive CTE: `anchor UNION step`, where `UNION` collapsing a node
/// already reached is what terminates a cyclic graph.
pub fn walk_cte(anchor: SelectBuilder, max_depth: i64) -> Cte {
  Cte::new("walk", anchor.union(walk_step(max_depth)))
    .columns(&["kind", "id", "depth"])
    .recursive()
}

/// The shallowest depth each reached node was found at, from any anchor.
pub fn walk_rows_from(anchor: SelectBuilder, max_depth: i64) -> SelectBuilder {
  let walk = walk_cte(anchor, max_depth);
  SelectBuilder::from_table(walk.table_ref())
    .column_as(&WALK_ID, "id")
    .column_scalar(Scalar::min(Scalar::col(&WALK_DEPTH)), "depth")
    .group_by(&WALK_ID)
    .order_by(WALK_ID.asc())
    .with(walk)
}

/// [`walk_rows_from`] over the shared seeds table.
pub fn walk_rows(batch: &str, max_depth: i64) -> SelectBuilder {
  walk_rows_from(seeded_anchor(batch), max_depth)
}

/// One `code_graph_nodes` lookup branch.
pub fn by_repo(repo: &str) -> SelectBuilder {
  SelectBuilder::new("code_symbols")
    .columns_qualified(&[&SYMBOL_ID])
    .filter(SYMBOL_REPO.eq(repo))
}

/// The other branch, overlapping the first on exactly one id.
pub fn by_path(path: &str) -> SelectBuilder {
  SelectBuilder::new("code_symbols")
    .columns_qualified(&[&SYMBOL_ID])
    .filter(SYMBOL_PATH.eq(path))
}

/// The ids one repo+path pair owns — nested into the `DELETE`, never read into
/// Rust.
pub fn ids_of_repo_path(repo: &str, path: &str) -> SelectBuilder {
  SelectBuilder::new("code_symbols")
    .columns_qualified(&[&SYMBOL_ID])
    .filter(SYMBOL_REPO.eq(repo))
    .filter(SYMBOL_PATH.eq(path))
}

/// The surviving `code_vec` rows, for asserting what a prune left behind.
pub fn remaining_vec_rows() -> SelectBuilder {
  SelectBuilder::new("code_vec")
    .columns_raw(&["symbol_id", "note"])
    .order_by(VEC_SYMBOL_ID.asc())
}

/// Items whose owner no longer exists, the NULL-safe way.
pub fn items_with_no_owner() -> SelectBuilder {
  items_listing().filter(Expr::not_exists(owner_of_item()))
}

/// The same question asked with `NOT IN`, whose answer differs whenever the
/// inner set — or the outer key — holds a NULL.
pub fn items_not_in_owner_ids(exclude_null_owners: bool) -> SelectBuilder {
  items_listing()
    .filter(Scalar::col(&ITEM_OWNER_ID).not_in_subquery(owner_ids(exclude_null_owners)))
}

/// The positive membership test.
pub fn items_in_owner_ids() -> SelectBuilder {
  items_listing().filter(Scalar::col(&ITEM_OWNER_ID).in_subquery(owner_ids(false)))
}

/// Every item with a correlated count of its owner rows, projected by a scalar
/// subquery.
pub fn items_with_owner_counts() -> SelectBuilder {
  items_listing().column_scalar(Scalar::subquery(owner_count_of_item()), "owner_rows")
}

fn items_listing() -> SelectBuilder {
  SelectBuilder::new("items")
    .column_as(&ITEM_ID, "id")
    .order_by(ITEM_ID.asc())
}

fn owner_ids(exclude_null: bool) -> SelectBuilder {
  let listing = SelectBuilder::new("owners").columns_qualified(&[&OWNER_ID]);
  if exclude_null {
    listing.filter(OWNER_ID.is_not_null())
  } else {
    listing
  }
}

fn owner_of_item() -> SelectBuilder {
  SelectBuilder::new("owners")
    .column_expr("1", "one")
    .filter(OWNER_ID.equals(&ITEM_OWNER_ID).into())
}

fn owner_count_of_item() -> SelectBuilder {
  SelectBuilder::new("owners")
    .column_scalar(Scalar::count_star(), "n")
    .filter(OWNER_ID.equals(&ITEM_OWNER_ID).into())
}
