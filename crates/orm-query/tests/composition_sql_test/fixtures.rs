//! The tables the composition scenarios render against, and the builders more
//! than one assertion shares.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::{Column, CommonOps, NumericOps};
use toolu_orm_core::value::Value;
use toolu_orm_query::select::{Cte, SelectBuilder};

pub const SYMBOL_ID: Column<Text> = Column::new("code_symbols", "id");
pub const REPO: Column<Text> = Column::new("code_symbols", "repo");
pub const PATH: Column<Text> = Column::new("code_symbols", "path");

pub const EDGE_SRC_KIND: Column<Text> = Column::new("edges", "src_kind");
pub const EDGE_SRC_ID: Column<Text> = Column::new("edges", "src_id");
pub const EDGE_DST_KIND: Column<Text> = Column::new("edges", "dst_kind");
pub const EDGE_DST_ID: Column<Text> = Column::new("edges", "dst_id");

pub const SEED_BATCH: Column<Text> = Column::new("walk_seeds", "batch");
pub const SEED_KIND: Column<Text> = Column::new("walk_seeds", "kind");
pub const SEED_ID: Column<Text> = Column::new("walk_seeds", "id");

pub const VEC_SYMBOL_ID: Column<Text> = Column::new("code_vec", "symbol_id");

pub const OWNER_ID: Column<Text> = Column::new("owners", "id");
pub const ITEM_ID: Column<Text> = Column::new("items", "id");
pub const ITEM_OWNER_ID: Column<Text> = Column::new("items", "owner_id");

/// `json_each` exposes a `value` column; the alias is what qualifies it.
pub const SEED_VALUE: Column<Text> = Column::new("json_each", "value");

pub const WALK_KIND: Column<Text> = Column::new("walk", "kind");
pub const WALK_ID: Column<Text> = Column::new("walk", "id");
pub const WALK_DEPTH: Column<Integer> = Column::new("walk", "depth");

/// `json_each(?N) AS "seeds"` over a two-element array.
pub fn seeds_source() -> TableRef {
  TableRef::function("json_each", vec![Value::Text(r#"["a","b"]"#.to_owned())])
    // `json_each` is a bare identifier, so this cannot fail. The fallback keeps
    // the fixture total; were it ever taken, the rendered SQL would lose the
    // call and every assertion below would fail loudly.
    .unwrap_or_else(|_| TableRef::new("json_each"))
    .with_alias("seeds")
}

/// The `code_graph_nodes` shape from the issue: one lookup branch by repo.
pub fn by_repo(repo: &str) -> SelectBuilder {
  SelectBuilder::new("code_symbols")
    .columns_qualified(&[&SYMBOL_ID])
    .filter(REPO.eq(repo))
}

/// The other branch: the same ids reached by path.
pub fn by_path(path: &str) -> SelectBuilder {
  SelectBuilder::new("code_symbols")
    .columns_qualified(&[&SYMBOL_ID])
    .filter(PATH.eq(path))
}

/// The `code_row.rs` shape: the ids one repo+path pair owns, never read into
/// Rust — only nested into the `DELETE` that uses them.
pub fn ids_of_repo_path(repo: &str, path: &str) -> SelectBuilder {
  SelectBuilder::new("code_symbols")
    .columns_qualified(&[&SYMBOL_ID])
    .filter(REPO.eq(repo))
    .filter(PATH.eq(path))
}

/// The recursive `walk` CTE from the issue: seeds at depth 0, then one hop per
/// iteration, `UNION` collapsing a node already reached so a cycle terminates,
/// and a bound depth limit.
pub fn recursive_walk(max_depth: i64) -> Cte {
  let seeds = TableRef::aliased("walk_seeds", "w");
  let edges = TableRef::aliased("edges", "e");
  let walk = TableRef::aliased("walk", "s");

  let anchor = SelectBuilder::from_table(&seeds)
    .column_as(&seeds.column(&SEED_KIND), "kind")
    .column_as(&seeds.column(&SEED_ID), "id")
    .column_scalar(Scalar::sql("CAST(0 AS BIGINT)"), "depth")
    .filter(seeds.column(&SEED_BATCH).eq("b1"));

  let step = SelectBuilder::from_table(&edges)
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
    .filter(walk.column(&WALK_DEPTH).lt(max_depth));

  Cte::new("walk", anchor.union(step))
    .columns(&["kind", "id", "depth"])
    .recursive()
}

/// The issue's outer read of the walk: the shallowest depth each node was
/// reached at.
pub fn walk_rows(max_depth: i64) -> SelectBuilder {
  let walk = recursive_walk(max_depth);
  SelectBuilder::from_table(walk.table_ref())
    .column_as(&WALK_ID, "id")
    .column_scalar(Scalar::min(Scalar::col(&WALK_DEPTH)), "depth")
    .group_by(&WALK_ID)
    .order_by(WALK_ID.asc())
    .with(walk)
}
