//! The vec0 KNN read surface as pure SQL: what `MATCH`, `k = ?` and
//! `distance` render, and everything they refuse.
//!
//! No database here — executing the SQL needs sqlite-vec on the connection,
//! which this workspace does not link (see
//! [vec0 KNN](../../../docs/scenarios/vec0-knn.md)).

mod rejections;
mod rendering;
