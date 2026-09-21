//! Expansion of the `row_security` field of the generated `TableDef`.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

use crate::parse::{PolicyInput, RowSecurityInput};

/// `None`, or a `RowSecurity` literal with one `PolicyDef` per attribute.
pub fn row_security_tokens(core: &TokenStream, input: Option<&RowSecurityInput>) -> TokenStream {
  let Some(security) = input else {
    return quote! { ::core::option::Option::None };
  };
  let force = security.force;
  let policies = security.policies.iter().map(|p| policy_def_tokens(core, p));
  quote! {
    ::core::option::Option::Some(#core::policy::RowSecurity {
      force: #force,
      policies: vec![#(#policies),*],
    })
  }
}

fn policy_def_tokens(core: &TokenStream, policy: &PolicyInput) -> TokenStream {
  let name = &policy.name;
  let kind = Ident::new(
    if policy.restrictive {
      "Restrictive"
    } else {
      "Permissive"
    },
    Span::call_site(),
  );
  let command = Ident::new(policy.command, Span::call_site());
  let roles = &policy.roles;
  let using = option_string_tokens(policy.using.as_deref());
  let with_check = option_string_tokens(policy.with_check.as_deref());
  quote! {
    #core::policy::PolicyDef {
      name: #name.to_owned(),
      kind: #core::policy::PolicyKind::#kind,
      command: #core::policy::PolicyCommand::#command,
      roles: vec![#(#roles.to_owned()),*],
      using: #using,
      with_check: #with_check,
    }
  }
}

fn option_string_tokens(value: Option<&str>) -> TokenStream {
  let Some(v) = value else {
    return quote! { ::core::option::Option::None };
  };
  quote! { ::core::option::Option::Some(#v.to_owned()) }
}
