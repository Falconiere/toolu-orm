//! The tables the query-composition scenarios execute against, and the rows
//! every driver seeds, so the SQLite and Postgres suites assert the same data.
//!
//! The rows are chosen so each acceptance criterion has a *discriminating*
//! answer. `edges` holds the cycle `a → b → c → a` plus the one-way `c → d`, so
//! an unterminated walk would loop forever and the depth limit alone decides
//! whether `d` is reached: `max = 0` gives one node, `2` gives three, `3` gives
//! four. `owners` holds a NULL id and `items` a NULL `owner_id`, which is what
//! separates `NOT EXISTS` from `NOT IN`. `code_symbols` splits `3` rows into
//! two branches that overlap on exactly one id, so `UNION` (3) differs from
//! `UNION ALL` (4) and from either arm (2 and 2).

use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;

pub const EDGE_SRC_KIND: Column<Text> = Column::new("edges", "src_kind");
pub const EDGE_SRC_ID: Column<Text> = Column::new("edges", "src_id");
pub const EDGE_DST_KIND: Column<Text> = Column::new("edges", "dst_kind");
pub const EDGE_DST_ID: Column<Text> = Column::new("edges", "dst_id");

pub const SEED_BATCH: Column<Text> = Column::new("walk_seeds", "batch");
pub const SEED_KIND: Column<Text> = Column::new("walk_seeds", "kind");
pub const SEED_ID: Column<Text> = Column::new("walk_seeds", "id");

/// `json_each` exposes a `value` column; an alias is what qualifies it.
pub const SEED_VALUE: Column<Text> = Column::new("json_each", "value");

pub const WALK_KIND: Column<Text> = Column::new("walk", "kind");
pub const WALK_ID: Column<Text> = Column::new("walk", "id");
pub const WALK_DEPTH: Column<Integer> = Column::new("walk", "depth");

pub const OWNER_ID: Column<Text> = Column::new("owners", "id");
pub const ITEM_ID: Column<Text> = Column::new("items", "id");
pub const ITEM_OWNER_ID: Column<Text> = Column::new("items", "owner_id");

pub const SYMBOL_ID: Column<Text> = Column::new("code_symbols", "id");
pub const SYMBOL_REPO: Column<Text> = Column::new("code_symbols", "repo");
pub const SYMBOL_PATH: Column<Text> = Column::new("code_symbols", "path");

pub const VEC_SYMBOL_ID: Column<Text> = Column::new("code_vec", "symbol_id");
pub const VEC_NOTE: Column<Text> = Column::new("code_vec", "note");

/// `(src_kind, src_id, dst_kind, dst_id)` — a three-node cycle plus one exit.
pub const EDGES_SEED: [(&str, &str, &str, &str); 4] = [
  ("memory", "a", "memory", "b"),
  ("memory", "b", "memory", "c"),
  ("memory", "c", "memory", "a"),
  ("memory", "c", "memory", "d"),
];

/// `(batch, kind, id)` — the walk's starting point, selected by a bound batch.
pub const WALK_SEEDS: [(&str, &str, &str); 2] = [("b1", "memory", "a"), ("b2", "memory", "d")];

/// The owner ids, one of them NULL.
pub const OWNERS_SEED: [Option<&str>; 2] = [Some("o1"), None];

/// `(id, owner_id)` — a matched owner, a missing one, and a NULL key.
pub const ITEMS_SEED: [(&str, Option<&str>); 3] =
  [("i1", Some("o1")), ("i2", Some("o2")), ("i3", None)];

/// `(id, repo, path)` — two branches overlapping on `c1`.
pub const SYMBOLS_SEED: [(&str, &str, &str); 3] = [
  ("c1", "r1", "src/a.rs"),
  ("c2", "r1", "src/b.rs"),
  ("c3", "r2", "src/a.rs"),
];

/// `(symbol_id, note)` — the rows a set-based `DELETE` prunes.
pub const VEC_SEED: [(&str, &str); 3] = [("c1", "v1"), ("c2", "v2"), ("c3", "v3")];

pub const EDGES_DDL: &str = "CREATE TABLE edges (src_kind TEXT NOT NULL, \
   src_id TEXT NOT NULL, dst_kind TEXT NOT NULL, dst_id TEXT NOT NULL)";
pub const WALK_SEEDS_DDL: &str = "CREATE TABLE walk_seeds (batch TEXT NOT NULL, \
   kind TEXT NOT NULL, id TEXT NOT NULL)";
pub const OWNERS_DDL: &str = "CREATE TABLE owners (id TEXT)";
pub const ITEMS_DDL: &str = "CREATE TABLE items (id TEXT PRIMARY KEY, owner_id TEXT)";
pub const SYMBOLS_DDL: &str = "CREATE TABLE code_symbols (id TEXT PRIMARY KEY, \
   repo TEXT NOT NULL, path TEXT NOT NULL)";
pub const VEC_DDL: &str = "CREATE TABLE code_vec (symbol_id TEXT NOT NULL, note TEXT NOT NULL)";

/// Every table, in creation order. Portable as written: `TEXT`, `PRIMARY KEY`
/// and `NOT NULL` mean the same thing on SQLite and Postgres.
pub const ALL_DDL: [&str; 6] = [
  EDGES_DDL,
  WALK_SEEDS_DDL,
  OWNERS_DDL,
  ITEMS_DDL,
  SYMBOLS_DDL,
  VEC_DDL,
];
