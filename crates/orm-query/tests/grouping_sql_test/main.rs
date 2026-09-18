//! `DISTINCT`, `GROUP BY` and `HAVING` rendering: the SQL text and the
//! parameter order, per dialect.
//!
//! Every assertion names its dialect explicitly, because this binary is listed
//! by both the default and the postgres lanes and `Dialect::CURRENT` differs
//! between them. The executed counterparts are the `*_grouping_test` binaries.

mod count_tests;
mod fixtures;
mod render_tests;
