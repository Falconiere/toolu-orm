//! Postgres `SelectBuilder` fetch methods.

use super::shared::impl_async_fetch;

impl_async_fetch!(toolu_orm_core::row::PgCountScalar);
