use super::support::{ELEMENTS, FINITE_BOUNDARIES, TestResult, invalid_inputs, value};
use toolu_orm_core::{value::to_pg_params, vector::Vector};

#[tokio::test]
async fn pgvector_round_trip_and_prewrite_validation() -> TestResult {
  let port = std::env::var("TEST_DB_PORT").unwrap_or_else(|_| "5434".into());
  let password = std::env::var("TEST_DB_PASSWORD").unwrap_or_else(|_| "toolu".into());
  let (client, connection) = tokio_postgres::Config::new()
    .host("localhost")
    .port(port.parse()?)
    .user("toolu")
    .password(password)
    .dbname("toolu")
    .connect(tokio_postgres::NoTls)
    .await?;
  let task = tokio::spawn(connection);
  client
    .batch_execute(
      "CREATE EXTENSION IF NOT EXISTS vector; CREATE TEMP TABLE vectors (embedding vector(3))",
    )
    .await?;
  let native = to_pg_params(&[value(&ELEMENTS)?])?;
  client
    .execute(
      "INSERT INTO vectors VALUES ($1)",
      &[native.first().ok_or("no parameter")?.as_ref()],
    )
    .await?;
  for invalid in invalid_inputs() {
    let result = async {
      let native = to_pg_params(&[value(&invalid)?])?;
      client
        .execute(
          "INSERT INTO vectors VALUES ($1)",
          &[native.first().ok_or("no parameter")?.as_ref()],
        )
        .await?;
      Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    assert!(result.is_err());
  }
  let row = client
    .query_one(
      "SELECT embedding, pg_typeof(embedding)::text, (SELECT count(*) FROM vectors) FROM vectors",
      &[],
    )
    .await?;
  assert_eq!(row.try_get::<_, Vector<3>>(0)?.as_slice(), ELEMENTS);
  assert_eq!(row.try_get::<_, String>(1)?, "vector");
  assert_eq!(row.try_get::<_, i64>(2)?, 1);
  assert!(row.try_get::<_, Vector<2>>(0).is_err());
  let row = client.query_one("SELECT NULL::vector", &[]).await?;
  assert!(row.try_get::<_, Option<Vector<3>>>(0)?.is_none());
  assert!(row.try_get::<_, Vector<3>>(0).is_err());
  for elements in FINITE_BOUNDARIES {
    let params = to_pg_params(&[value(&elements)?])?;
    let row = client
      .query_one(
        "SELECT $1::vector",
        &[params.first().ok_or("no parameter")?.as_ref()],
      )
      .await?;
    assert_eq!(row.try_get::<_, Vector<3>>(0)?.as_slice(), elements);
  }
  drop(client);
  task.await??;
  Ok(())
}
