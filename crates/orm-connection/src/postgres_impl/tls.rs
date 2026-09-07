//! TLS configuration for Postgres connections using rustls and native certs.

use std::sync::Arc;

use crate::error::DbError;

pub(crate) fn make_rustls_config() -> Result<rustls::ClientConfig, DbError> {
  let mut root_store = rustls::RootCertStore::empty();
  let loaded = rustls_native_certs::load_native_certs();
  for err in loaded.errors {
    tracing::debug!(%err, "native cert load warning");
  }
  for cert in loaded.certs {
    root_store
      .add(cert)
      .map_err(|e| DbError::Connection(format!("invalid TLS root certificate: {e}")))?;
  }
  if root_store.is_empty() {
    return Err(DbError::Connection(
      "TLS root store is empty after loading native certificates".to_owned(),
    ));
  }
  Ok(
    rustls::ClientConfig::builder()
      .with_root_certificates(Arc::new(root_store))
      .with_no_client_auth(),
  )
}
