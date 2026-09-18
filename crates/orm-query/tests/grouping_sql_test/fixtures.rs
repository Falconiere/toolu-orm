//! The two tables the grouping scenarios render against, and the builders
//! shared by more than one assertion.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_query::select::SelectBuilder;

pub const SOURCE_ID: Column<Text> = Column::new("source_files", "source_id");
pub const PATH: Column<Text> = Column::new("source_files", "path");
pub const STATUS: Column<Text> = Column::new("source_files", "status");
pub const SIZE_BYTES: Column<Integer> = Column::new("source_files", "size_bytes");

pub const SOURCE_PK: Column<Text> = Column::new("sources", "id");
pub const LABEL: Column<Text> = Column::new("sources", "label");

/// `source_files AS "f"`.
pub fn files() -> TableRef {
  TableRef::aliased("source_files", "f")
}

/// `sources AS "s"`.
pub fn sources() -> TableRef {
  TableRef::aliased("sources", "s")
}

/// The production shape from the issue: one row per status, with its count.
pub fn status_counts() -> SelectBuilder {
  SelectBuilder::new("source_files")
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .filter(SOURCE_ID.eq("s1"))
    .group_by(&STATUS)
}

/// The other production shape: one row per distinct path.
///
/// Projected qualified so the `ORDER BY` term is textually a select-list item,
/// which is what Postgres requires of a `DISTINCT` query.
pub fn distinct_paths() -> SelectBuilder {
  SelectBuilder::new("source_files")
    .columns_qualified(&[&PATH])
    .distinct()
    .filter(SOURCE_ID.eq("s1"))
    .order_by(PATH.asc())
}
