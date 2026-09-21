//! The generated policy filters rows and rejects writes for a non-owner role
//! whose tenant was set with `set_local_config`, and lets nothing through
//! when no tenant was set.

use toolu_orm_connection::{DbConnection, DbError};
use toolu_orm_core::value::Value;

use super::rls_registry::{migrations_dir, registry_v1};
use super::support::{count, generate_and_migrate, grant_app_role, pg_schema_conn, TestResult};

const ROLE: &str = "toolu_rls_app";
const COUNT_DOCS: &str = "SELECT count(*) FROM docs";
const INSERT_DOC: &str = "INSERT INTO docs (id, tenant_id, title) VALUES ($1, $2, $3)";

fn doc(id: &str, tenant: i64) -> Vec<Value> {
  vec![
    Value::Text(id.to_owned()),
    Value::Integer(tenant),
    Value::Text(format!("doc {id}")),
  ]
}

#[tokio::test]
async fn policy_filters_reads_and_writes_per_tenant() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, mut conn) = pg_schema_conn("rls_enforce").await?;
  generate_and_migrate(&conn, &dir, &registry_v1(), "init").await?;
  grant_app_role(&conn, ROLE).await?;

  // Seeded by the owner, who row security does not bind (not FORCEd).
  for (id, tenant) in [("a", 1), ("b", 1), ("c", 2)] {
    conn.execute_sql(INSERT_DOC, doc(id, tenant)).await?;
  }
  assert_eq!(count(&conn, COUNT_DOCS).await?, 3);

  // Tenant 1 sees its two rows and can add a third of its own.
  let tx = conn.transaction().await?;
  tx.execute_batch(&format!("SET LOCAL ROLE {ROLE}")).await?;
  tx.set_local_config("app.tenant_id", "1").await?;
  assert_eq!(count(&tx, COUNT_DOCS).await?, 2);
  assert_eq!(tx.execute_sql(INSERT_DOC, doc("d", 1)).await?, 1);
  assert_eq!(count(&tx, COUNT_DOCS).await?, 3);
  tx.commit().await?;

  // Tenant 2 sees one row, and a row for another tenant fails WITH CHECK
  // (the USING expression, since the policy is FOR ALL without WITH CHECK).
  let tx = conn.transaction().await?;
  tx.execute_batch(&format!("SET LOCAL ROLE {ROLE}")).await?;
  tx.set_local_config("app.tenant_id", "2").await?;
  assert_eq!(count(&tx, COUNT_DOCS).await?, 1);
  let err = tx
    .execute_sql(INSERT_DOC, doc("e", 1))
    .await
    .err()
    .ok_or("a cross-tenant insert should be rejected")?;
  assert!(
    matches!(&err, DbError::Query(msg) if msg.contains("row-level security") && msg.contains("42501")),
    "err: {err}"
  );
  drop(tx);

  // No tenant set: the role sees nothing at all.
  let tx = conn.transaction().await?;
  tx.execute_batch(&format!("SET LOCAL ROLE {ROLE}")).await?;
  assert_eq!(count(&tx, COUNT_DOCS).await?, 0);
  drop(tx);

  // Back on the owner's session: every row, including tenant 1's new one.
  assert_eq!(count(&conn, COUNT_DOCS).await?, 4);
  Ok(())
}
