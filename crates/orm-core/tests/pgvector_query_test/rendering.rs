//! What distance expressions render, including all three operators.

use toolu_orm_core::column::Vector;
use toolu_orm_core::dialect::Dialect;
use toolu_orm_core::pgvector::{self, DistanceOp, PgVectorOps};
use toolu_orm_core::query_column::Column;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EMBEDDING: Column<Vector> = Column::new("items", "embedding");

#[test]
fn l2_embeds_the_query_vector_as_a_literal() -> TestResult {
  let dist = EMBEDDING.l2_distance_for(Dialect::Postgres, &[1.0, 0.0])?;
  assert_eq!(dist.sql(), r#""items"."embedding" <-> '[1,0]'::vector"#);
  assert_eq!(
    dist.asc().to_sql(),
    r#""items"."embedding" <-> '[1,0]'::vector ASC"#
  );
  assert_eq!(
    dist.desc().to_sql(),
    r#""items"."embedding" <-> '[1,0]'::vector DESC"#
  );
  Ok(())
}

#[test]
fn cosine_and_neg_inner_product_emit_their_ops() -> TestResult {
  let cosine = EMBEDDING.cosine_distance_for(Dialect::Postgres, &[0.5, 0.5])?;
  assert_eq!(
    cosine.sql(),
    r#""items"."embedding" <=> '[0.5,0.5]'::vector"#
  );
  let nip = EMBEDDING.neg_inner_product_for(Dialect::Postgres, &[1.0, 2.0, 3.0])?;
  assert_eq!(nip.sql(), r#""items"."embedding" <#> '[1,2,3]'::vector"#);
  let via_enum = EMBEDDING.distance_for(Dialect::Postgres, DistanceOp::L2, &[1.0])?;
  assert_eq!(via_enum.sql(), r#""items"."embedding" <-> '[1]'::vector"#);
  Ok(())
}

#[test]
fn free_functions_agree_with_the_ops_trait() -> TestResult {
  let via_trait = EMBEDDING.l2_distance_for(Dialect::Postgres, &[1.0, 2.0])?;
  let via_fn = pgvector::l2_distance_for(Dialect::Postgres, &EMBEDDING, &[1.0, 2.0])?;
  assert_eq!(via_trait.sql(), via_fn.sql());
  Ok(())
}

#[test]
fn the_short_forms_agree_with_the_current_dialect() {
  let short = EMBEDDING.l2_distance(&[1.0, 0.0]);
  let explicit = EMBEDDING.l2_distance_for(Dialect::CURRENT, &[1.0, 0.0]);
  assert_eq!(short.is_ok(), explicit.is_ok());
  if let (Ok(a), Ok(b)) = (short, explicit) {
    assert_eq!(a.sql(), b.sql());
  }
}

#[test]
fn extreme_finite_floats_never_use_scientific_notation() -> TestResult {
  let dist = EMBEDDING.l2_distance_for(Dialect::Postgres, &[1e-7, 1e20, f32::MIN_POSITIVE])?;
  let sql = dist.sql();
  let start = sql.find('\'').ok_or("missing opening quote")?;
  let end = sql.rfind('\'').ok_or("missing closing quote")?;
  let literal = sql.get(start..=end).ok_or("literal slice out of range")?;
  assert!(
    !literal.contains(['e', 'E']),
    "pgvector literal must stay decimal-only, got {literal}"
  );
  assert!(sql.ends_with("::vector") || sql.contains("'::vector"));
  Ok(())
}
