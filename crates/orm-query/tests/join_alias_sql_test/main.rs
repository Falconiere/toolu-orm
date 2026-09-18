//! `SelectBuilder` with table aliases, qualified projections and expression
//! `ON` clauses: the SQL text and the parameter order, per dialect.
//!
//! Every assertion names its dialect explicitly, because this binary is listed
//! by both the default and the postgres lanes and `Dialect::CURRENT` differs
//! between them. The executed counterparts are the `*_joins_test` binaries.

mod alias_tests;
mod fixtures;
mod param_order_tests;
