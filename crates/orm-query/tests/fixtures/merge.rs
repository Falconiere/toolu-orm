use toolu_orm_core::{
  column::{Integer, Text},
  query_column::Column,
  value::Value,
};
use toolu_orm_query::merge::{Matched, MergeBuilder, NotMatched};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const ID: Column<Integer> = Column::new("items", "id");
pub const LABEL: Column<Text> = Column::new("items", "label");
pub const SCORE: Column<Integer> = Column::new("items", "score");
pub const HOSTILE: &str = "O'Brien ?1 $2; DROP TABLE items; -- 東京";

pub fn batch() -> MergeBuilder {
  MergeBuilder::new("items")
    .columns(&[&ID, &LABEL, &SCORE])
    .keys(&[&ID])
    .when_matched(Matched::Update)
    .when_not_matched(NotMatched::Insert)
}

pub fn row(id: i64, label: &str, score: i64) -> Vec<Value> {
  vec![id.into(), label.into(), score.into()]
}
