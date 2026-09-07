use toolu_orm_core::rename::{NoRenames, RenameResolver};

#[test]
fn no_renames_returns_empty_table_renames() {
  let resolver = NoRenames;
  let added = vec!["accounts".to_owned()];
  let removed = vec!["users".to_owned()];
  let result = resolver.resolve_tables(&added, &removed);
  assert!(result.is_empty());
}

#[test]
fn no_renames_returns_empty_column_renames() {
  let resolver = NoRenames;
  let added = vec!["full_name".to_owned()];
  let removed = vec!["name".to_owned()];
  let result = resolver.resolve_columns("users", &added, &removed);
  assert!(result.is_empty());
}

#[test]
fn no_renames_with_empty_inputs() {
  let resolver = NoRenames;
  let result = resolver.resolve_tables(&[], &[]);
  assert!(result.is_empty());
  let result = resolver.resolve_columns("any_table", &[], &[]);
  assert!(result.is_empty());
}
