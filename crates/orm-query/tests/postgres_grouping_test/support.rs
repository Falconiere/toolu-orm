//! The builders the rusqlite grouping scenarios share.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::select::SelectBuilder;

use crate::seed::{PATH, SOURCE_ID, STATUS};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `SELECT DISTINCT "source_files"."path" … WHERE source_id = 's1'
/// ORDER BY path ASC`.
///
/// Projected qualified so the `ORDER BY` term is textually a select-list item
/// — what Postgres requires of a `DISTINCT` query, and harmless on SQLite.
pub fn distinct_paths() -> SelectBuilder {
  SelectBuilder::new("source_files")
    .columns_qualified(&[&PATH])
    .distinct()
    .filter(SOURCE_ID.eq("s1"))
    .order_by(PATH.asc())
}

/// The same listing without `DISTINCT`, which is what makes the duplicate rows
/// visible.
pub fn raw_paths() -> SelectBuilder {
  SelectBuilder::new("source_files")
    .columns_qualified(&[&PATH])
    .filter(SOURCE_ID.eq("s1"))
    .order_by(PATH.asc())
}

/// `SELECT status, COUNT(*) AS "n" … GROUP BY status`, over every row.
pub fn status_counts() -> SelectBuilder {
  SelectBuilder::new("source_files")
    .columns_raw(&["status"])
    .column_scalar(Scalar::count_star(), "n")
    .group_by(&STATUS)
}

/// `source_files AS "f"`.
pub fn files() -> TableRef {
  TableRef::aliased("source_files", "f")
}

/// `sources AS "s"`.
pub fn sources() -> TableRef {
  TableRef::aliased("sources", "s")
}
