use toolu_orm_core::column::{Integer, Text};
use toolu_orm_core::query_column::Column;

pub const ID: Column<Text> = Column::new("users", "id");
pub const EMAIL: Column<Text> = Column::new("users", "email");
pub const ORG_ID: Column<Text> = Column::new("users", "org_id");
pub const CREATED_AT: Column<Integer> = Column::new("users", "created_at");

pub const PIPELINE_USER_ID: Column<Text> = Column::new("pipelines", "user_id");
