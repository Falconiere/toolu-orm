//! Parsing of table-level `#[foreign_key(...)]` attributes on a `#[table]` struct.
//!
//! ```ignore
//! #[foreign_key(
//!     name = "project_evidence_work_item_project_fk",
//!     columns(work_item_id, project_id),
//!     references = "project_work_items(id, project_id)",
//!     on_delete = "cascade",
//! )]
//! ```
//!
//! `columns` and `references` are required and name at least two columns
//! each, pairwise; `name` defaults to `fk_<table>_<columns joined by _>`;
//! `on_delete` / `on_update` take the `#[column]` vocabulary.

use syn::meta::ParseNestedMeta;
use syn::punctuated::Punctuated;
use syn::{Attribute, Error, Ident, ItemStruct, LitStr, Result, Token};

/// One parsed `#[foreign_key(...)]`.
pub struct ForeignKeyInput {
  /// The explicit `name`, kept as a literal for error spans.
  pub name: Option<LitStr>,
  pub columns: Vec<Ident>,
  pub references_table: String,
  pub references_columns: Vec<String>,
  pub on_delete: Option<String>,
  pub on_update: Option<String>,
  /// The attribute itself, for errors that belong to no single key.
  pub attr: Attribute,
}

impl ForeignKeyInput {
  /// The constraint name: the explicit one, or `fk_<table>_<columns>`.
  pub fn name(&self, table_name: &str) -> String {
    if let Some(name) = &self.name {
      return name.value();
    }
    let columns: Vec<String> = self.columns.iter().map(Ident::to_string).collect();
    format!("fk_{table_name}_{}", columns.join("_"))
  }
}

const EXPECTED_KEY: &str =
  "expected `name = \"…\"`, `columns(…)`, `references = \"…\"`, `on_delete = \"…\"` or `on_update = \"…\"`";
const EXPECTED_REFERENCES: &str = "expected `references = \"table(col, …)\"`";
const EXPECTED_ACTION: &str =
  "expected `cascade`, `set_null`, `set_default`, `restrict` or `no_action`";
const ACTIONS: [&str; 5] = [
  "cascade",
  "set_null",
  "set_default",
  "restrict",
  "no_action",
];

/// Extracts and strips every `#[foreign_key(...)]` from the struct, in
/// declaration order.
pub fn parse_foreign_key_attrs(item: &mut ItemStruct) -> Result<Vec<ForeignKeyInput>> {
  let mut foreign_keys = Vec::new();
  let mut remaining_attrs = Vec::new();
  for attr in &item.attrs {
    if attr.path().is_ident("foreign_key") {
      foreign_keys.push(parse_foreign_key_attr(attr)?);
    } else {
      remaining_attrs.push(attr.clone());
    }
  }
  item.attrs = remaining_attrs;
  Ok(foreign_keys)
}

/// The keys of one attribute, each at most once.
#[derive(Default)]
struct Keys {
  name: Option<LitStr>,
  columns: Option<Vec<Ident>>,
  references: Option<LitStr>,
  on_delete: Option<LitStr>,
  on_update: Option<LitStr>,
}

fn parse_foreign_key_attr(attr: &Attribute) -> Result<ForeignKeyInput> {
  let mut keys = Keys::default();
  attr.parse_nested_meta(|meta| apply_key(&meta, &mut keys))?;
  let columns = keys
    .columns
    .ok_or_else(|| Error::new_spanned(attr, "#[foreign_key] needs `columns(a, b)`"))?;
  let references = keys.references.ok_or_else(|| {
    Error::new_spanned(
      attr,
      "#[foreign_key] needs `references = \"table(col, …)\"`",
    )
  })?;
  let (references_table, references_columns) = parse_references(&references)?;
  if columns.len() < 2 {
    return Err(Error::new_spanned(
      attr,
      "#[foreign_key] needs at least two columns; declare a single-column foreign key with #[column(references = \"table(col)\")]",
    ));
  }
  if columns.len() != references_columns.len() {
    return Err(Error::new_spanned(
      &references,
      format!(
        "`columns(...)` lists {} columns but `references` names {}",
        columns.len(),
        references_columns.len()
      ),
    ));
  }
  Ok(ForeignKeyInput {
    name: keys.name,
    columns,
    references_table,
    references_columns,
    on_delete: keys.on_delete.map(|l| l.value()),
    on_update: keys.on_update.map(|l| l.value()),
    attr: attr.clone(),
  })
}

fn apply_key(meta: &ParseNestedMeta<'_>, keys: &mut Keys) -> Result<()> {
  if meta.path.is_ident("columns") {
    if keys.columns.is_some() {
      return Err(meta.error("duplicate `columns` in #[foreign_key]"));
    }
    let content;
    syn::parenthesized!(content in meta.input);
    let list = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
    keys.columns = Some(list.into_iter().collect());
    return Ok(());
  }
  let (key, slot) = if meta.path.is_ident("name") {
    ("name", &mut keys.name)
  } else if meta.path.is_ident("references") {
    ("references", &mut keys.references)
  } else if meta.path.is_ident("on_delete") {
    ("on_delete", &mut keys.on_delete)
  } else if meta.path.is_ident("on_update") {
    ("on_update", &mut keys.on_update)
  } else {
    return Err(meta.error(EXPECTED_KEY));
  };
  if slot.is_some() {
    return Err(meta.error(format!("duplicate `{key}` in #[foreign_key]")));
  }
  let lit: LitStr = meta.value()?.parse()?;
  let is_action = key == "on_delete" || key == "on_update";
  if is_action && !ACTIONS.contains(&lit.value().as_str()) {
    return Err(Error::new_spanned(&lit, EXPECTED_ACTION));
  }
  *slot = Some(lit);
  Ok(())
}

/// Splits `"table(a, b)"` into the table and its trimmed column names.
fn parse_references(lit: &LitStr) -> Result<(String, Vec<String>)> {
  let value = lit.value();
  let invalid = || Error::new_spanned(lit, EXPECTED_REFERENCES);
  let (table, rest) = value.split_once('(').ok_or_else(invalid)?;
  let inner = rest.strip_suffix(')').ok_or_else(invalid)?;
  let table = table.trim();
  let columns: Vec<String> = inner.split(',').map(|c| c.trim().to_owned()).collect();
  if table.is_empty() || columns.iter().any(String::is_empty) {
    return Err(invalid());
  }
  Ok((table.to_owned(), columns))
}
