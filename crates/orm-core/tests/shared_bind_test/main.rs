//! The reusable-binding surface at expression level: handle identity, reuse
//! across nested `AND` / `OR`, the empty list, and how a shared placeholder
//! interacts with raw fragments and with an explicit fragment offset.
//!
//! Every assertion names its dialect: this binary lists in both the default
//! and the postgres lane, where `Dialect::CURRENT` differs.

mod fixtures;
mod foreign_source;
mod identity;
mod nesting;
mod raw_and_offsets;
