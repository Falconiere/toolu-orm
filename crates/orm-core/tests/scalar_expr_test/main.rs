//! `Scalar` expression rendering: leaves, function calls, aggregates,
//! arithmetic, comparisons, `CASE`, and `LIKE … ESCAPE`, per dialect and per
//! offset.

mod aggregates;
mod arithmetic;
mod case_terms;
mod comparisons;
mod functions;
mod leaves;
