//! Absolute paths to the crates the expansions name, as this consumer can name them.
//!
//! Everything the macros emit is rooted at `::` (or `crate`) so it resolves
//! from anywhere — including the companion column module `#[table]` generates,
//! where a `use` in the parent module does not apply.
//!
//! Which root depends on the consumer's `Cargo.toml`, resolved by
//! [`proc_macro_crate`]:
//!
//! | Direct dependency | Emitted |
//! |---|---|
//! | `toolu-orm-core` | `::toolu_orm_core` |
//! | `toolu-orm` only | `::toolu_orm::core` |
//! | neither, or no Cargo context | `::toolu_orm_core` |
//!
//! Resolution never fails an expansion: without a readable manifest it falls
//! back to the bare crate name, which is what a direct dependent has anyway.

use std::sync::OnceLock;

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;

const CORE_PKG: &str = "toolu-orm-core";
const QUERY_PKG: &str = "toolu-orm-query";
const FACADE_PKG: &str = "toolu-orm";

/// A resolved crate root plus the optional facade module that re-exports it.
struct CratePath {
  root: Root,
  module: Option<&'static str>,
}

enum Root {
  /// The crate being expanded is the target crate itself.
  Itself,
  /// An extern crate, under the name this consumer declared it with.
  Extern(String),
}

impl CratePath {
  fn tokens(&self) -> TokenStream {
    let base = match &self.root {
      Root::Itself => quote! { crate },
      Root::Extern(name) => {
        let ident = Ident::new(name, Span::call_site());
        quote! { ::#ident }
      },
    };
    match self.module {
      Some(module) => {
        let ident = Ident::new(module, Span::call_site());
        quote! { #base::#ident }
      },
      None => base,
    }
  }

  fn to_literal(&self) -> String {
    let base = match &self.root {
      Root::Itself => "crate".to_owned(),
      Root::Extern(name) => format!("::{name}"),
    };
    match self.module {
      Some(module) => format!("{base}::{module}"),
      None => base,
    }
  }
}

/// Resolves `direct` as a direct dependency, else the facade's `module`
/// re-export, else `direct`'s own crate name.
fn resolve(direct: &str, module: &'static str) -> CratePath {
  if let Some(root) = found(direct) {
    return CratePath { root, module: None };
  }
  if let Some(root) = found(FACADE_PKG) {
    return CratePath {
      root,
      module: Some(module),
    };
  }
  CratePath {
    root: Root::Extern(sanitize(direct)),
    module: None,
  }
}

fn found(package: &str) -> Option<Root> {
  match crate_name(package) {
    Ok(FoundCrate::Itself) => Some(Root::Itself),
    Ok(FoundCrate::Name(name)) => Some(Root::Extern(sanitize(&name))),
    Err(_) => None,
  }
}

fn sanitize(name: &str) -> String {
  name.replace('-', "_")
}

fn core_path() -> &'static CratePath {
  static CORE: OnceLock<CratePath> = OnceLock::new();
  CORE.get_or_init(|| resolve(CORE_PKG, "core"))
}

/// Absolute path to `toolu-orm-core`.
pub fn core() -> TokenStream {
  core_path().tokens()
}

/// Absolute path to `toolu-orm-query`.
pub fn query() -> TokenStream {
  static QUERY: OnceLock<CratePath> = OnceLock::new();
  QUERY.get_or_init(|| resolve(QUERY_PKG, "query")).tokens()
}

/// The core path as a string, for `#[serde(crate = "...")]` on generated
/// structs: serde's derive emits `extern crate serde as _serde`, which needs
/// `serde` in the extern prelude unless it is told where to find it.
pub fn core_serde_literal() -> String {
  format!("{}::serde", core_path().to_literal())
}
