//! v1 → v2 → v3 through real generate/migrate calls, checked in the catalog.

use toolu_orm_cli::status::get_status;
use toolu_orm_core::dialect::Dialect;

use super::rls_registry::{migrations_dir, registry_v1, registry_v2, registry_v3};
use super::support::{generate_and_migrate, pg_schema_conn, policies_where, rls_flags, TestResult};

#[tokio::test]
async fn loop_applies_enable_policies_force_and_disable() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = pg_schema_conn("rls_loop").await?;

  assert_eq!(
    generate_and_migrate(&conn, &dir, &registry_v1(), "init").await?,
    1
  );
  assert_eq!(rls_flags(&conn).await?, (true, false));
  assert_eq!(policies_where(&conn, "true").await?, 1);
  assert_eq!(
    policies_where(
      &conn,
      "policyname = 'tenant_isolation' AND permissive = 'PERMISSIVE' AND cmd = 'ALL' \
       AND roles = '{public}' AND qual LIKE '%app.tenant_id%' AND with_check IS NULL"
    )
    .await?,
    1
  );

  assert_eq!(
    generate_and_migrate(&conn, &dir, &registry_v2(), "restrict").await?,
    1
  );
  assert_eq!(rls_flags(&conn).await?, (true, true));
  assert_eq!(policies_where(&conn, "true").await?, 2);
  assert_eq!(
    policies_where(
      &conn,
      "policyname = 'hide_archived' AND permissive = 'RESTRICTIVE' AND cmd = 'SELECT' \
       AND qual = '(NOT archived)'"
    )
    .await?,
    1
  );

  assert_eq!(
    generate_and_migrate(&conn, &dir, &registry_v3(), "open").await?,
    1
  );
  assert_eq!(rls_flags(&conn).await?, (false, false));
  assert_eq!(policies_where(&conn, "true").await?, 0);

  let status = get_status(&conn, &dir, Dialect::Postgres).await?;
  assert_eq!(status.applied.len(), 3);
  assert!(status.pending.is_empty());
  Ok(())
}

#[tokio::test]
async fn unchanged_declaration_generates_nothing() -> TestResult {
  let (_tmp, dir) = migrations_dir()?;
  let (_db, conn) = pg_schema_conn("rls_noop").await?;
  generate_and_migrate(&conn, &dir, &registry_v2(), "init").await?;
  let again =
    toolu_orm_cli::generate::run_generate(&registry_v2(), &dir, "again", Dialect::Postgres)?;
  assert_eq!(again, None);
  Ok(())
}
