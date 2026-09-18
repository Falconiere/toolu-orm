//! JSON access fragments (`json_extract(...)`) and the comparisons they carry.

use crate::query_column::Column;
use crate::value::Value;

use super::predicate::Expr;

/// A JSON access fragment usable inside expressions.
pub struct JsonExpr {
  fragment: String,
}

impl Expr {
  pub fn json_extract<T>(col: &Column<T>, path: &str) -> JsonExpr {
    let fragment = format!(r#"json_extract({}, '{}')"#, col.qualified(), path);
    JsonExpr { fragment }
  }
}

impl JsonExpr {
  pub fn eq<V: Into<Value>>(self, val: V) -> Expr {
    Expr::comparison(self.fragment, "=", val.into())
  }

  pub fn like<V: Into<Value>>(self, val: V) -> Expr {
    Expr::comparison(self.fragment, "LIKE", val.into())
  }
}
