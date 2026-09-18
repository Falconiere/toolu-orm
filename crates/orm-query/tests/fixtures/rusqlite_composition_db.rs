//! In-memory rusqlite database for the query-composition scenarios, seeded
//! through `InsertBuilder`.

use toolu_orm_macros::FromRow;
use toolu_orm_query::insert::InsertBuilder;

use super::seed::{
  ALL_DDL, EDGES_SEED, EDGE_DST_ID, EDGE_DST_KIND, EDGE_SRC_ID, EDGE_SRC_KIND, ITEMS_SEED, ITEM_ID,
  ITEM_OWNER_ID, OWNERS_SEED, OWNER_ID, SEED_BATCH, SEED_ID, SEED_KIND, SYMBOLS_SEED, SYMBOL_ID,
  SYMBOL_PATH, SYMBOL_REPO, VEC_NOTE, VEC_SEED, VEC_SYMBOL_ID, WALK_SEEDS,
};

/// One node of a walk and the shallowest depth it was reached at.
#[derive(FromRow, Debug, PartialEq)]
pub struct WalkRow {
  pub id: String,
  pub depth: i64,
}

/// A single projected id.
#[derive(FromRow, Debug, PartialEq)]
pub struct IdRow {
  pub id: String,
}

/// An item and the number of owner rows its key matches.
#[derive(FromRow, Debug, PartialEq)]
pub struct OwnerCountRow {
  pub id: String,
  pub owner_rows: i64,
}

/// A surviving `code_vec` row.
#[derive(FromRow, Debug, PartialEq)]
pub struct VecRow {
  pub symbol_id: String,
  pub note: String,
}

/// One element of a `json_each` expansion.
#[derive(FromRow, Debug, PartialEq)]
pub struct ValueRow {
  pub value: String,
}

/// One column name from `pragma_table_info`.
#[derive(FromRow, Debug, PartialEq)]
pub struct NameRow {
  pub name: String,
}

/// Creates every table and inserts the seed rows.
///
/// # Errors
///
/// The underlying rusqlite or builder error.
pub fn setup_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  for ddl in ALL_DDL {
    conn.execute(ddl, ())?;
  }
  for (src_kind, src_id, dst_kind, dst_id) in EDGES_SEED {
    InsertBuilder::new("edges")
      .set(&EDGE_SRC_KIND, src_kind)
      .set(&EDGE_SRC_ID, src_id)
      .set(&EDGE_DST_KIND, dst_kind)
      .set(&EDGE_DST_ID, dst_id)
      .execute(&conn)?;
  }
  for (batch, kind, id) in WALK_SEEDS {
    InsertBuilder::new("walk_seeds")
      .set(&SEED_BATCH, batch)
      .set(&SEED_KIND, kind)
      .set(&SEED_ID, id)
      .execute(&conn)?;
  }
  for owner in OWNERS_SEED {
    let insert = InsertBuilder::new("owners");
    match owner {
      Some(id) => insert.set(&OWNER_ID, id).execute(&conn)?,
      None => insert.set_null(&OWNER_ID).execute(&conn)?,
    };
  }
  for (id, owner_id) in ITEMS_SEED {
    let insert = InsertBuilder::new("items").set(&ITEM_ID, id);
    match owner_id {
      Some(owner) => insert.set(&ITEM_OWNER_ID, owner).execute(&conn)?,
      None => insert.set_null(&ITEM_OWNER_ID).execute(&conn)?,
    };
  }
  for (id, repo, path) in SYMBOLS_SEED {
    InsertBuilder::new("code_symbols")
      .set(&SYMBOL_ID, id)
      .set(&SYMBOL_REPO, repo)
      .set(&SYMBOL_PATH, path)
      .execute(&conn)?;
  }
  for (symbol_id, note) in VEC_SEED {
    InsertBuilder::new("code_vec")
      .set(&VEC_SYMBOL_ID, symbol_id)
      .set(&VEC_NOTE, note)
      .execute(&conn)?;
  }
  Ok(conn)
}

/// `(id, depth)` pairs, for order-sensitive assertions.
pub fn walk_pairs(rows: &[WalkRow]) -> Vec<(&str, i64)> {
  rows
    .iter()
    .map(|row| (row.id.as_str(), row.depth))
    .collect()
}

/// The ids of `rows`, for order-sensitive assertions.
pub fn ids(rows: &[IdRow]) -> Vec<&str> {
  rows.iter().map(|row| row.id.as_str()).collect()
}
