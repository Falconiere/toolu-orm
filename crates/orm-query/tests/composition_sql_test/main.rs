//! Rendering and bind order for the query-composition surface: `WITH` /
//! `WITH RECURSIVE`, `UNION` / `UNION ALL`, subqueries in predicate and scalar
//! position, and table-valued `FROM` sources.
//!
//! Every assertion names its dialect explicitly, because this binary is listed
//! by both the default and the postgres lanes and `Dialect::CURRENT` differs
//! between them. The executed counterparts are the `*_composition_test`
//! binaries.

mod bind_order;
mod cte;
mod fixtures;
mod set_ops;
mod subquery;
mod table_functions;
