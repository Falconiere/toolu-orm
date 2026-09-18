//! `InsertBuilder`'s explicit conflict clause and `RETURNING` projection,
//! rendered per dialect: the clause itself, where its placeholders land in the
//! statement's one ordered parameter list, and the projected columns.
//!
//! Driver-free, so this binary runs in the default lane *and* in the postgres
//! lane, where `Dialect::CURRENT` is Postgres. Every assertion therefore names
//! its dialect with `to_sql_for`; none calls `to_sql()`.

mod binding;
mod clause;
mod columns;
mod returning;
