//! Parse `#[has_many]`, `#[belongs_to]`, `#[many_to_many]` on struct fields.
//!
//! # Public API
//!
//! - [`RelationInput`], [`is_relation_field`], [`parse_relation_attr`]

use syn::{Attribute, Expr, ExprLit, Field, Lit};

/// Kind of relation from attributes.
#[derive(Debug, Clone)]
pub enum RelationInputKind {
  HasMany,
  BelongsTo,
  ManyToMany {
    through_table: String,
    local_key: String,
  },
}

/// Parsed relation on a field.
#[derive(Debug, Clone)]
pub struct RelationInput {
  /// Struct field name (result alias).
  pub field_name: String,
  pub kind: RelationInputKind,
  pub target_table: String,
  pub foreign_key: String,
  /// Target row columns (JSON array order → struct fields).
  pub columns: Vec<String>,
  pub inner_type: syn::Ident,
}

/// Whether the field carries a relation attribute.
pub fn is_relation_field(field: &Field) -> bool {
  field.attrs.iter().any(|attr| {
    attr.path().is_ident("has_many")
      || attr.path().is_ident("belongs_to")
      || attr.path().is_ident("many_to_many")
  })
}

/// Parse the first relation attribute on `field`, if any.
pub fn parse_relation_attr(field: &Field) -> syn::Result<Option<RelationInput>> {
  let field_name = field
    .ident
    .as_ref()
    .ok_or_else(|| syn::Error::new_spanned(field, "relation fields must be named"))?
    .to_string();

  for attr in &field.attrs {
    if attr.path().is_ident("has_many") {
      return parse_has_many(attr, &field_name, field).map(Some);
    }
    if attr.path().is_ident("belongs_to") {
      return parse_belongs_to(attr, &field_name, field).map(Some);
    }
    if attr.path().is_ident("many_to_many") {
      return parse_many_to_many(attr, &field_name, field).map(Some);
    }
  }
  Ok(None)
}

fn extract_inner_type(field: &Field) -> syn::Result<syn::Ident> {
  let syn::Type::Path(type_path) = &field.ty else {
    return Err(syn::Error::new_spanned(
      &field.ty,
      "relation field must be Vec<T> or Option<T>",
    ));
  };
  let segment = type_path.path.segments.last().ok_or_else(|| {
    syn::Error::new_spanned(&field.ty, "relation field must be Vec<T> or Option<T>")
  })?;
  let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
    return Err(syn::Error::new_spanned(
      &field.ty,
      "relation field must be Vec<T> or Option<T>",
    ));
  };
  let syn::GenericArgument::Type(syn::Type::Path(inner_path)) = args
    .args
    .first()
    .ok_or_else(|| syn::Error::new_spanned(&field.ty, "missing generic type argument"))?
  else {
    return Err(syn::Error::new_spanned(
      &field.ty,
      "generic argument must be a type path",
    ));
  };
  let ident = inner_path
    .path
    .segments
    .last()
    .ok_or_else(|| syn::Error::new_spanned(&field.ty, "cannot resolve inner type name"))?;
  Ok(ident.ident.clone())
}

fn parse_string_columns(expr: Expr) -> syn::Result<Vec<String>> {
  let Expr::Array(arr) = expr else {
    return Err(syn::Error::new_spanned(
      expr,
      "columns must be a string array [...]",
    ));
  };
  if arr.elems.is_empty() {
    return Err(syn::Error::new_spanned(
      Expr::Array(arr),
      "columns must not be empty",
    ));
  }
  let mut out = Vec::new();
  for elem in arr.elems {
    let Expr::Lit(ExprLit {
      lit: Lit::Str(s), ..
    }) = elem
    else {
      return Err(syn::Error::new_spanned(elem, "expected string literal"));
    };
    out.push(s.value());
  }
  Ok(out)
}

fn parse_table_fk_and_columns(attr: &Attribute) -> syn::Result<(String, String, Vec<String>)> {
  let mut table = None::<String>;
  let mut foreign_key = None::<String>;
  let mut columns = None::<Vec<String>>;

  attr.parse_nested_meta(|meta| {
    if meta.path.is_ident("table") {
      let v: syn::LitStr = meta.value()?.parse()?;
      table = Some(v.value());
    } else if meta.path.is_ident("foreign_key") {
      let v: syn::LitStr = meta.value()?.parse()?;
      foreign_key = Some(v.value());
    } else if meta.path.is_ident("columns") {
      let expr: Expr = meta.value()?.parse()?;
      columns = Some(parse_string_columns(expr)?);
    } else {
      return Err(meta.error("unknown attribute"));
    }
    Ok(())
  })?;

  let table = table.ok_or_else(|| syn::Error::new_spanned(attr, "missing `table`"))?;
  let fk = foreign_key.ok_or_else(|| syn::Error::new_spanned(attr, "missing `foreign_key`"))?;
  let columns = columns.ok_or_else(|| syn::Error::new_spanned(attr, "missing `columns`"))?;
  Ok((table, fk, columns))
}

fn parse_has_many(attr: &Attribute, field_name: &str, field: &Field) -> syn::Result<RelationInput> {
  let (table, fk, columns) = parse_table_fk_and_columns(attr)?;
  let inner_type = extract_inner_type(field)?;
  Ok(RelationInput {
    field_name: field_name.to_owned(),
    kind: RelationInputKind::HasMany,
    target_table: table,
    foreign_key: fk,
    columns,
    inner_type,
  })
}

fn parse_belongs_to(
  attr: &Attribute,
  field_name: &str,
  field: &Field,
) -> syn::Result<RelationInput> {
  let (table, fk, columns) = parse_table_fk_and_columns(attr)?;
  let inner_type = extract_inner_type(field)?;
  Ok(RelationInput {
    field_name: field_name.to_owned(),
    kind: RelationInputKind::BelongsTo,
    target_table: table,
    foreign_key: fk,
    columns,
    inner_type,
  })
}

fn parse_many_to_many(
  attr: &Attribute,
  field_name: &str,
  field: &Field,
) -> syn::Result<RelationInput> {
  let inner_type = extract_inner_type(field)?;
  let mut table = None::<String>;
  let mut through = None::<String>;
  let mut local_key = None::<String>;
  let mut foreign_key = None::<String>;
  let mut columns = None::<Vec<String>>;

  attr.parse_nested_meta(|meta| {
    if meta.path.is_ident("table") {
      let v: syn::LitStr = meta.value()?.parse()?;
      table = Some(v.value());
    } else if meta.path.is_ident("through") {
      let v: syn::LitStr = meta.value()?.parse()?;
      through = Some(v.value());
    } else if meta.path.is_ident("local_key") {
      let v: syn::LitStr = meta.value()?.parse()?;
      local_key = Some(v.value());
    } else if meta.path.is_ident("foreign_key") {
      let v: syn::LitStr = meta.value()?.parse()?;
      foreign_key = Some(v.value());
    } else if meta.path.is_ident("columns") {
      let expr: Expr = meta.value()?.parse()?;
      columns = Some(parse_string_columns(expr)?);
    } else {
      return Err(meta.error("unknown attribute"));
    }
    Ok(())
  })?;

  let table = table.ok_or_else(|| syn::Error::new_spanned(attr, "missing `table`"))?;
  let through = through.ok_or_else(|| syn::Error::new_spanned(attr, "missing `through`"))?;
  let lk = local_key.ok_or_else(|| syn::Error::new_spanned(attr, "missing `local_key`"))?;
  let fk = foreign_key.ok_or_else(|| syn::Error::new_spanned(attr, "missing `foreign_key`"))?;
  let columns = columns.ok_or_else(|| syn::Error::new_spanned(attr, "missing `columns`"))?;

  Ok(RelationInput {
    field_name: field_name.to_owned(),
    kind: RelationInputKind::ManyToMany {
      through_table: through,
      local_key: lk,
    },
    target_table: table,
    foreign_key: fk,
    columns,
    inner_type,
  })
}
