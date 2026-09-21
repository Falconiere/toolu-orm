//! Parsing of `#[policy(...)]` attributes on a `#[table]` struct.
//!
//! ```ignore
//! #[policy("tenant_isolation", for = select, as = restrictive,
//!          to = ["app_user"], using = "tenant_id = current_setting('app.tenant_id')::int")]
//! ```
//!
//! The first argument is the policy name; every other argument is a
//! `key = value` pair. `for` takes `all`, `select`, `insert`, `update` or
//! `delete`; `as` takes `permissive` or `restrictive`; `to` takes one role
//! string or a bracketed list of them; `using` and `with_check` take the raw
//! SQL expression as a string.

use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, ItemStruct, LitStr, Result, Token};

/// One parsed `#[policy(...)]`.
pub struct PolicyInput {
  pub name: String,
  /// `restrictive` when set; permissive otherwise.
  pub restrictive: bool,
  /// The `PolicyCommand` variant name: `All`, `Select`, `Insert`, `Update`
  /// or `Delete`.
  pub command: &'static str,
  pub roles: Vec<String>,
  pub using: Option<String>,
  pub with_check: Option<String>,
}

/// Row security as the attributes declared it.
pub struct RowSecurityInput {
  pub force: bool,
  pub policies: Vec<PolicyInput>,
}

const EXPECTED_KEY: &str =
  "expected `for = …`, `as = …`, `to = …`, `using = \"…\"` or `with_check = \"…\"`";

/// The keys `#[policy(...)]` accepts after the name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PolicyKey {
  For,
  As,
  To,
  Using,
  WithCheck,
}

impl PolicyKey {
  const fn name(self) -> &'static str {
    match self {
      Self::For => "for",
      Self::As => "as",
      Self::To => "to",
      Self::Using => "using",
      Self::WithCheck => "with_check",
    }
  }
}

/// Extracts and strips every `#[policy(...)]` from the struct, in order.
pub fn parse_policy_attrs(item: &mut ItemStruct) -> Result<Vec<PolicyInput>> {
  let mut policies: Vec<PolicyInput> = Vec::new();
  let mut remaining_attrs = Vec::new();
  for attr in &item.attrs {
    if !attr.path().is_ident("policy") {
      remaining_attrs.push(attr.clone());
      continue;
    }
    let policy = parse_policy_attr(attr)?;
    if policies.iter().any(|p| p.name == policy.name) {
      return Err(syn::Error::new_spanned(
        attr,
        format!("duplicate policy \"{}\" on the same table", policy.name),
      ));
    }
    policies.push(policy);
  }
  item.attrs = remaining_attrs;
  Ok(policies)
}

/// Rejects `#[policy]` on a struct that cannot carry one (a virtual table).
pub fn reject_policy_attrs(item: &ItemStruct, macro_name: &str) -> Result<()> {
  for attr in &item.attrs {
    if attr.path().is_ident("policy") {
      return Err(syn::Error::new_spanned(
        attr,
        format!("virtual tables cannot declare policies; remove it from #[{macro_name}]"),
      ));
    }
  }
  Ok(())
}

fn parse_policy_attr(attr: &Attribute) -> Result<PolicyInput> {
  attr.parse_args::<PolicyInput>()
}

impl Parse for PolicyInput {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    if !input.peek(LitStr) {
      return Err(syn::Error::new(
        input.span(),
        "first arg must be the policy name string",
      ));
    }
    let name: LitStr = input.parse()?;
    let mut policy = Self {
      name: name.value(),
      restrictive: false,
      command: "All",
      roles: Vec::new(),
      using: None,
      with_check: None,
    };
    let mut seen: Vec<PolicyKey> = Vec::new();
    while !input.is_empty() {
      input.parse::<Token![,]>()?;
      if input.is_empty() {
        break;
      }
      let key = parse_key(input, &mut seen)?;
      input.parse::<Token![=]>()?;
      match key {
        PolicyKey::For => policy.command = parse_command(input)?,
        PolicyKey::As => policy.restrictive = parse_kind(input)?,
        PolicyKey::To => policy.roles = parse_roles(input)?,
        PolicyKey::Using => policy.using = Some(input.parse::<LitStr>()?.value()),
        PolicyKey::WithCheck => policy.with_check = Some(input.parse::<LitStr>()?.value()),
      }
    }
    Ok(policy)
  }
}

/// The next key, rejecting one already given and one that is not a key.
fn parse_key(input: ParseStream<'_>, seen: &mut Vec<PolicyKey>) -> Result<PolicyKey> {
  let span = input.span();
  let key = if input.peek(Token![for]) {
    input.parse::<Token![for]>()?;
    PolicyKey::For
  } else if input.peek(Token![as]) {
    input.parse::<Token![as]>()?;
    PolicyKey::As
  } else if input.peek(Ident) {
    let ident: Ident = input.parse()?;
    match ident.to_string().as_str() {
      "to" => PolicyKey::To,
      "using" => PolicyKey::Using,
      "with_check" => PolicyKey::WithCheck,
      _ => return Err(syn::Error::new(ident.span(), EXPECTED_KEY)),
    }
  } else {
    return Err(syn::Error::new(span, EXPECTED_KEY));
  };
  if seen.contains(&key) {
    return Err(syn::Error::new(
      span,
      format!("duplicate `{}` on policy", key.name()),
    ));
  }
  seen.push(key);
  Ok(key)
}

/// `for = select` → the `PolicyCommand` variant name, so the expansion never
/// has to derive an identifier from user text.
fn parse_command(input: ParseStream<'_>) -> Result<&'static str> {
  let ident: Ident = input.parse()?;
  match ident.to_string().as_str() {
    "all" => Ok("All"),
    "select" => Ok("Select"),
    "insert" => Ok("Insert"),
    "update" => Ok("Update"),
    "delete" => Ok("Delete"),
    _ => Err(syn::Error::new(
      ident.span(),
      "expected `for = all`, `select`, `insert`, `update` or `delete`",
    )),
  }
}

fn parse_kind(input: ParseStream<'_>) -> Result<bool> {
  let ident: Ident = input.parse()?;
  match ident.to_string().as_str() {
    "permissive" => Ok(false),
    "restrictive" => Ok(true),
    _ => Err(syn::Error::new(
      ident.span(),
      "expected `as = permissive` or `as = restrictive`",
    )),
  }
}

/// `to = "role"` or `to = ["role", "other"]`.
fn parse_roles(input: ParseStream<'_>) -> Result<Vec<String>> {
  if input.peek(LitStr) {
    return Ok(vec![input.parse::<LitStr>()?.value()]);
  }
  if !input.peek(syn::token::Bracket) {
    return Err(syn::Error::new(
      input.span(),
      "expected `to = \"role\"` or `to = [\"role\", …]`",
    ));
  }
  let content;
  syn::bracketed!(content in input);
  let roles = content.parse_terminated(|s| s.parse::<LitStr>(), Token![,])?;
  let roles: Vec<String> = roles.iter().map(LitStr::value).collect();
  if roles.is_empty() {
    return Err(syn::Error::new(
      content.span(),
      "`to = […]` needs at least one role",
    ));
  }
  Ok(roles)
}
