//! Row-security diffs: enable with a new table, add / change / drop a policy,
//! flip `FORCE`, disable, survive a rename, and the declarations `diff`
//! refuses before writing anything.

mod changes;
mod refusals;
mod support;
