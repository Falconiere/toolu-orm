//! `Scalar` expression rendering: leaves, function calls, arithmetic,
//! comparisons, `CASE`, and `LIKE … ESCAPE`, per dialect and per offset.

mod arithmetic;
mod case_terms;
mod comparisons;
mod functions;
mod leaves;
