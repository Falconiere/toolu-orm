#![cfg(feature = "postgres")]

use std::time::Duration;

use toolu_orm_connection::postgres_impl::PgConfig;

#[test]
fn pg_config_default_port() {
  let config = PgConfig {
    host: "localhost".to_owned(),
    port: 5432,
    user: "toolu".to_owned(),
    password: "secret".to_owned(),
    dbname: "toolu".to_owned(),
    max_connections: 10,
    ssl: false,
    checkout_timeout: Some(Duration::from_secs(5)),
  };
  assert_eq!(config.port, 5432);
  assert_eq!(config.max_connections, 10);
}

#[test]
fn pg_config_custom_values() {
  let config = PgConfig {
    host: "db.example.com".to_owned(),
    port: 5433,
    user: "admin".to_owned(),
    password: "p@ssw0rd".to_owned(),
    dbname: "production".to_owned(),
    max_connections: 50,
    ssl: true,
    checkout_timeout: None,
  };
  assert_eq!(config.host, "db.example.com");
  assert_eq!(config.port, 5433);
  assert_eq!(config.dbname, "production");
  assert_eq!(config.max_connections, 50);
  assert_eq!(config.checkout_timeout, None);
}

#[test]
fn pg_config_for_test_uses_default_checkout_timeout() {
  let config = PgConfig::for_test("suite");
  assert_eq!(
    config.checkout_timeout,
    Some(PgConfig::DEFAULT_CHECKOUT_TIMEOUT)
  );
}
