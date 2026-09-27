#[path = "fixtures/merge.rs"]
pub mod merge;
use merge::{batch, row, TestResult, HOSTILE};
use toolu_orm_connection::{DbConnection, PgConfig, PgDatabase};
use toolu_orm_core::value::Value;
use toolu_orm_macros::FromRow;
use toolu_orm_query::merge::{Matched, NotMatched};

#[derive(Debug, PartialEq, FromRow)]
struct Stored {
  id: i64,
  label: Option<String>,
  score: i64,
}

#[tokio::test]
async fn native_merge_policies_nulls_duplicates_and_atomicity() -> TestResult {
  let db = PgDatabase::init(&PgConfig::for_test("toolu")).await?;
  let conn = db.connect().await?;
  let schema = format!("merge_177_{}", std::process::id());
  conn.execute_batch(&format!("CREATE SCHEMA {schema}; SET search_path TO {schema}; CREATE TABLE items(id BIGINT,label VARCHAR,score BIGINT CHECK(score >= 0)); INSERT INTO items VALUES(1,'old',10),(3,'untouched',30)")).await?;
  let outcome = async {
    assert_eq!(
      batch()
        .row(row(1, HOSTILE, 11))
        .row(row(2, "new", 20))
        .execute_on(&conn)
        .await?,
      2
    );
    assert_eq!(
      batch()
        .when_matched(Matched::DoNothing)
        .row(row(1, "ignored", 12))
        .row(row(4, "new", 40))
        .execute_on(&conn)
        .await?,
      1
    );
    assert_eq!(
      batch()
        .when_not_matched(NotMatched::DoNothing)
        .row(vec![2.into(), Value::Null, 21.into()])
        .row(row(9, "absent", 90))
        .execute_on(&conn)
        .await?,
      1
    );
    let before: Vec<Stored> = conn
      .query_map("SELECT * FROM items ORDER BY id", vec![])
      .await?;
    assert_eq!(
      before,
      vec![
        Stored {
          id: 1,
          label: Some(HOSTILE.into()),
          score: 11
        },
        Stored {
          id: 2,
          label: None,
          score: 21
        },
        Stored {
          id: 3,
          label: Some("untouched".into()),
          score: 30
        },
        Stored {
          id: 4,
          label: Some("new".into()),
          score: 40
        }
      ]
    );
    let error = batch()
      .row(row(1, "partial", 100))
      .row(row(5, "invalid", -1))
      .execute_on(&conn)
      .await
      .expect_err("constraint");
    assert!(error.to_string().contains("check constraint"), "{error}");
    assert_eq!(
      conn
        .query_map::<Stored>("SELECT * FROM items ORDER BY id", vec![])
        .await?,
      before
    );
    for id in [1, 9] {
      assert!(batch()
        .row(row(id, "a", 1))
        .row(row(id, "b", 2))
        .execute_on(&conn)
        .await
        .is_err());
    }
    conn
      .execute_batch("INSERT INTO items VALUES(1,'duplicate',50)")
      .await?;
    assert_eq!(batch().row(row(1, "same", 60)).execute_on(&conn).await?, 2);
    let duplicate: Vec<Stored> = conn
      .query_map("SELECT * FROM items WHERE id=1", vec![])
      .await?;
    assert_eq!(
      duplicate,
      vec![
        Stored {
          id: 1,
          label: Some("same".into()),
          score: 60
        },
        Stored {
          id: 1,
          label: Some("same".into()),
          score: 60
        }
      ]
    );
    Ok(()) as TestResult
  }
  .await;
  conn
    .execute_batch(&format!("DROP SCHEMA {schema} CASCADE"))
    .await?;
  outcome
}
