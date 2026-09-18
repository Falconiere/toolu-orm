//! Rendering and bind order for the reusable-binding surface: the issue's own
//! parameter counts, sharing across clause and statement boundaries, and the
//! mutation builders.
//!
//! Every assertion names its dialect explicitly, because this binary is listed
//! by both the default and the postgres lanes and `Dialect::CURRENT` differs
//! between them. The executed counterparts are the `*_reusable_bind_test`
//! binaries.

#[path = "../fixtures/reusable_bind_seed.rs"]
pub mod seed;

mod clauses;
mod issue_case;
mod nesting;
