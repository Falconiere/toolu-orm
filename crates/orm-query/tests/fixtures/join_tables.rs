//! Schema, seed, typed columns, decoded row shapes, and the builders for the
//! alias/join scenario suites, shared by the rusqlite, libsql and Postgres
//! binaries. Nothing here names a driver: `DDL` and `SEED` are portable SQL and
//! every query is a `SelectBuilder`, so all three lanes prove the same shapes.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::column::{BigInt, Text};
use toolu_orm_core::query_column::{Column, CommonOps};
use toolu_orm_macros::FromRow;
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

pub const S_ID: Column<Text> = Column::new("code_symbols", "id");
pub const S_OWNER: Column<Text> = Column::new("code_symbols", "owner");
pub const S_KIND: Column<Text> = Column::new("code_symbols", "kind");
pub const S_CREATED_AT: Column<BigInt> = Column::new("code_symbols", "created_at");

pub const F_ID: Column<Text> = Column::new("code_feedback", "id");
pub const F_SYMBOL_ID: Column<Text> = Column::new("code_feedback", "symbol_id");
pub const F_LIVE: Column<BigInt> = Column::new("code_feedback", "live");
pub const F_LABEL: Column<Text> = Column::new("code_feedback", "label");

/// Both tables carry an `id`, which is what makes an unqualified projection
/// ambiguous once they are joined. One statement per entry, because libsql
/// takes them one at a time.
pub const DDL: [&str; 2] = [
  "CREATE TABLE code_symbols (id TEXT PRIMARY KEY, owner TEXT NOT NULL, kind TEXT NOT NULL, \
   created_at BIGINT NOT NULL)",
  "CREATE TABLE code_feedback (id TEXT PRIMARY KEY, symbol_id TEXT NOT NULL, live BIGINT NOT NULL, \
   label TEXT NOT NULL)",
];

/// `s4` is the row the LEFT JOIN scenario turns on: its only feedback row is
/// `live = 0`, so an `ON` predicate keeps `s4` with a NULL label while the same
/// predicate in `WHERE` drops it.
pub const SEED: [&str; 2] = [
  "INSERT INTO code_symbols VALUES \
   ('s1','o1','note',10),('s2','o1','note',20),('s3','o2','task',30),('s4','o1','note',40)",
  "INSERT INTO code_feedback VALUES \
   ('f1','s1',1,'up'),('f2','s2',1,'down'),('f3','s3',1,'up'),('f4','s4',0,'stale')",
];

/// Every statement, for a driver that takes a whole script.
pub fn script() -> String {
  DDL
    .iter()
    .chain(SEED.iter())
    .copied()
    .collect::<Vec<_>>()
    .join("; ")
}

/// `code_symbols` under `alias`.
pub fn symbols(alias: &str) -> TableRef {
  TableRef::aliased("code_symbols", alias)
}

/// `code_feedback` under `alias`.
pub fn feedback(alias: &str) -> TableRef {
  TableRef::aliased("code_feedback", alias)
}

#[derive(FromRow, Debug, PartialEq, Eq)]
pub struct Pair {
  pub old_id: String,
  pub newer_id: String,
}

#[derive(FromRow, Debug, PartialEq, Eq)]
pub struct Labels {
  pub s_id: String,
  pub up_label: Option<String>,
  pub down_label: Option<String>,
}

#[derive(FromRow, Debug, PartialEq, Eq)]
pub struct Ids {
  pub s_id: String,
  pub f_id: Option<String>,
}

#[derive(FromRow, Debug, PartialEq, Eq)]
pub struct Labelled {
  pub s_id: String,
  pub label: Option<String>,
}

/// One table under two aliases, joined on an equality plus an inequality.
pub fn self_join_query() -> SelectBuilder {
  let old = symbols("old");
  let newer = symbols("newer");
  SelectBuilder::from_table(&old)
    .column_as(&old.column(&S_ID), "old_id")
    .column_as(&newer.column(&S_ID), "newer_id")
    .join(
      &newer,
      old.column(&S_OWNER).equals(&newer.column(&S_OWNER)).and(
        old
          .column(&S_CREATED_AT)
          .less_than(&newer.column(&S_CREATED_AT)),
      ),
    )
    .order_by(old.column(&S_ID).asc())
    .order_by(newer.column(&S_ID).asc())
}

/// The same table joined twice, each `ON` clause binding its own value — the
/// second join's placeholder has to continue after the first's.
pub fn two_joins_query() -> SelectBuilder {
  let c = symbols("c");
  let up = feedback("up");
  let down = feedback("down");
  SelectBuilder::from_table(&c)
    .column_as(&c.column(&S_ID), "s_id")
    .column_as(&up.column(&F_LABEL), "up_label")
    .column_as(&down.column(&F_LABEL), "down_label")
    .left_join(
      &up,
      up.column(&F_SYMBOL_ID)
        .equals(&c.column(&S_ID))
        .and(up.column(&F_LABEL).eq("up")),
    )
    .left_join(
      &down,
      down
        .column(&F_SYMBOL_ID)
        .equals(&c.column(&S_ID))
        .and(down.column(&F_LABEL).eq("down")),
    )
    .order_by(c.column(&S_ID).asc())
}

/// Bare names across two joined tables that both have an `id`: the database
/// rejects this, which is the defect `columns_qualified` exists to fix.
pub fn ambiguous_query() -> SelectBuilder {
  joined_pair().columns_typed(&[&S_ID, &F_LABEL])
}

/// The same join, projected qualified and under distinct output aliases.
pub fn qualified_ids_query() -> SelectBuilder {
  let c = symbols("c");
  let f = feedback("f");
  joined_pair()
    .column_as(&c.column(&S_ID), "s_id")
    .column_as(&f.column(&F_ID), "f_id")
}

/// `LEFT JOIN … ON fk = pk AND live = ?`, which keeps an unmatched row.
pub fn left_join_on_query(live: i64) -> SelectBuilder {
  let c = symbols("c");
  let f = feedback("f");
  SelectBuilder::from_table(&c)
    .column_as(&c.column(&S_ID), "s_id")
    .column_as(&f.column(&F_LABEL), "label")
    .left_join(
      &f,
      f.column(&F_SYMBOL_ID)
        .equals(&c.column(&S_ID))
        .and(f.column(&F_LIVE).eq(live)),
    )
    .order_by(c.column(&S_ID).asc())
}

/// The same predicate moved to `WHERE`, which drops it.
pub fn left_join_where_query(live: i64) -> SelectBuilder {
  let c = symbols("c");
  let f = feedback("f");
  joined_pair()
    .column_as(&c.column(&S_ID), "s_id")
    .column_as(&f.column(&F_LABEL), "label")
    .filter(f.column(&F_LIVE).eq(live))
    .order_by(c.column(&S_ID).asc())
}

/// One value bound in `ON`, another in `WHERE`.
pub fn on_and_where_query(live: i64, kind: &str) -> SelectBuilder {
  let c = symbols("c");
  left_join_on_query(live).filter(c.column(&S_KIND).eq(kind))
}

/// `code_symbols c LEFT JOIN code_feedback f ON f.symbol_id = c.id`, unprojected.
fn joined_pair() -> SelectBuilder {
  let c = symbols("c");
  let f = feedback("f");
  SelectBuilder::from_table(&c).left_join(&f, f.column(&F_SYMBOL_ID).equals(&c.column(&S_ID)))
}

/// The ids a query returned, for comparing against an expected list.
pub fn ids(rows: &[Labelled]) -> Vec<&str> {
  rows.iter().map(|r| r.s_id.as_str()).collect()
}

/// The labels a query returned, `None` for an unmatched row.
pub fn labels(rows: &[Labelled]) -> Vec<Option<&str>> {
  rows.iter().map(|r| r.label.as_deref()).collect()
}

/// A driver error's whole source chain as one string.
///
/// `tokio_postgres::Error` displays as the bare `"db error"` and keeps the
/// server's own message — "column reference \"id\" is ambiguous" — in its
/// source, so asserting on a message means walking the chain. The two SQLite
/// drivers put theirs in the top-level message; both are covered by joining
/// every link.
pub fn error_text(err: &QueryError) -> String {
  let mut text = err.to_string();
  let mut source = std::error::Error::source(err);
  while let Some(cause) = source {
    text.push_str(" / ");
    text.push_str(&cause.to_string());
    source = cause.source();
  }
  text
}
