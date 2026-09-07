//! Tests for typed column operations and expression generation.
//!
//! # Public API
//!
//! Tests are organized by concern:
//! - `common_ops` — eq, ne, is_null, is_not_null, in_list, not_in, and/or
//! - `numeric_ops` — gt, lt, lte, gte, between, offset numbering
//! - `text_and_special_ops` — like, json_extract, raw, varchar, date, time

mod common_ops;
mod numeric_ops;
mod text_and_special_ops;
