use toolu_orm_core::column::ColumnDef;
use toolu_orm_core::dialect::Dialect;

#[test]
fn sqlite_param_uses_question_mark() {
  assert_eq!(Dialect::Sqlite.param(1), "?1");
  assert_eq!(Dialect::Sqlite.param(5), "?5");
}

#[test]
fn postgres_param_uses_dollar_sign() {
  assert_eq!(Dialect::Postgres.param(1), "$1");
  assert_eq!(Dialect::Postgres.param(5), "$5");
}

#[test]
fn sqlite_map_default_passes_through_unixepoch() {
  let result = Dialect::Sqlite.map_default("unixepoch()");
  assert_eq!(result, "unixepoch()");
}

#[test]
fn postgres_map_default_translates_unixepoch() {
  let result = Dialect::Postgres.map_default("unixepoch()");
  assert_eq!(result, "extract(epoch from now())::bigint");
}

#[test]
fn sqlite_map_default_passes_through_uuid4_str() {
  let result = Dialect::Sqlite.map_default("uuid4_str()");
  assert_eq!(result, "uuid4_str()");
}

#[test]
fn postgres_map_default_translates_uuid4_str() {
  let result = Dialect::Postgres.map_default("uuid4_str()");
  assert_eq!(result, "gen_random_uuid()");
}

#[test]
fn map_default_passes_through_unknown_defaults() {
  assert_eq!(Dialect::Sqlite.map_default("42"), "42");
  assert_eq!(Dialect::Postgres.map_default("42"), "42");
  assert_eq!(Dialect::Sqlite.map_default("'draft'"), "'draft'");
  assert_eq!(Dialect::Postgres.map_default("'draft'"), "'draft'");
}

#[test]
fn sqlite_quote_ident_uses_double_quotes() {
  assert_eq!(Dialect::Sqlite.quote_ident("users"), r#""users""#);
  assert_eq!(Dialect::Sqlite.quote_ident("created_at"), r#""created_at""#);
}

#[test]
fn postgres_quote_ident_uses_double_quotes() {
  assert_eq!(Dialect::Postgres.quote_ident("users"), r#""users""#);
  assert_eq!(
    Dialect::Postgres.quote_ident("created_at"),
    r#""created_at""#
  );
}

#[test]
fn current_returns_sqlite_without_postgres_feature() {
  #[cfg(not(feature = "postgres"))]
  assert_eq!(Dialect::CURRENT, Dialect::Sqlite);
}

#[test]
#[cfg(feature = "postgres")]
fn current_returns_postgres_with_postgres_feature() {
  assert_eq!(Dialect::CURRENT, Dialect::Postgres);
}

#[test]
fn dialect_is_copy() {
  let d = Dialect::Sqlite;
  let d2 = d;
  assert_eq!(d, d2);
  assert_eq!(d, Dialect::Sqlite);
}

#[test]
fn dialect_debug_format() {
  assert_eq!(format!("{:?}", Dialect::Sqlite), "Sqlite");
  assert_eq!(format!("{:?}", Dialect::Postgres), "Postgres");
}

// ── Default mapping with column context ──────────────────────────────────────

#[test]
fn postgres_maps_timestamp_default_in_column_def() {
  let col = ColumnDef {
    name: "created_at".to_owned(),
    column_type: toolu_orm_core::column::ColumnType::Timestamp,
    primary_key: false,
    not_null: true,
    default: Some("unixepoch()".to_owned()),
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
  };
  assert_eq!(
    col
      .default
      .as_deref()
      .map(|d| Dialect::Postgres.map_default(d)),
    Some("extract(epoch from now())::bigint".to_owned())
  );
}

#[test]
fn sqlite_preserves_timestamp_default_in_column_def() {
  let col = ColumnDef {
    name: "created_at".to_owned(),
    column_type: toolu_orm_core::column::ColumnType::Timestamp,
    primary_key: false,
    not_null: true,
    default: Some("unixepoch()".to_owned()),
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
  };
  assert_eq!(
    col
      .default
      .as_deref()
      .map(|d| Dialect::Sqlite.map_default(d)),
    Some("unixepoch()".to_owned())
  );
}

#[test]
fn postgres_maps_uuid_default_in_column_def() {
  let col = ColumnDef {
    name: "id".to_owned(),
    column_type: toolu_orm_core::column::ColumnType::Uuid,
    primary_key: true,
    not_null: false,
    default: Some("uuid4_str()".to_owned()),
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
  };
  assert_eq!(
    col
      .default
      .as_deref()
      .map(|d| Dialect::Postgres.map_default(d)),
    Some("gen_random_uuid()".to_owned())
  );
}

#[test]
fn sqlite_preserves_uuid_default_in_column_def() {
  let col = ColumnDef {
    name: "id".to_owned(),
    column_type: toolu_orm_core::column::ColumnType::Uuid,
    primary_key: true,
    not_null: false,
    default: Some("uuid4_str()".to_owned()),
    unique: false,
    references: None,
    on_delete: None,
    on_update: None,
    check: None,
    unindexed: false,
  };
  assert_eq!(
    col
      .default
      .as_deref()
      .map(|d| Dialect::Sqlite.map_default(d)),
    Some("uuid4_str()".to_owned())
  );
}

// ── Edge cases ───────────────────────────────────────────────────────────────

#[test]
fn postgres_passes_through_string_literals() {
  assert_eq!(Dialect::Postgres.map_default("'active'"), "'active'");
}

#[test]
fn postgres_passes_through_numeric_defaults() {
  assert_eq!(Dialect::Postgres.map_default("0"), "0");
  assert_eq!(Dialect::Postgres.map_default("100"), "100");
}

#[test]
fn postgres_passes_through_boolean_defaults() {
  assert_eq!(Dialect::Postgres.map_default("true"), "true");
  assert_eq!(Dialect::Postgres.map_default("false"), "false");
}

#[test]
fn postgres_passes_through_null_keyword() {
  assert_eq!(Dialect::Postgres.map_default("NULL"), "NULL");
}

#[test]
fn postgres_passes_through_empty_string_literal() {
  assert_eq!(Dialect::Postgres.map_default("''"), "''");
}

#[test]
fn sqlite_passes_through_all_non_function_defaults() {
  assert_eq!(Dialect::Sqlite.map_default("'active'"), "'active'");
  assert_eq!(Dialect::Sqlite.map_default("0"), "0");
  assert_eq!(Dialect::Sqlite.map_default("true"), "true");
  assert_eq!(Dialect::Sqlite.map_default("NULL"), "NULL");
}
