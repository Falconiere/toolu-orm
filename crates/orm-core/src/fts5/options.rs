//! FTS5 module options and their rendering into `key = 'value'` arguments.

/// The `fts5(...)` settings that follow the column list.
///
/// `tokenize` is a free string: it holds a whole tokenizer chain
/// (`porter unicode61 remove_diacritics 2`) and may name a tokenizer the
/// application registered itself, so there is no closed set to validate
/// against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fts5Options {
  pub prefix: Option<String>,
  pub tokenize: Option<String>,
  pub content: Option<String>,
  pub content_rowid: Option<String>,
  pub columnsize: Option<u8>,
  pub detail: Option<String>,
}

impl Fts5Options {
  /// Rendered in a fixed order, so the same schema always produces the same
  /// argument list and an unchanged schema never looks changed to the diff.
  #[must_use]
  pub fn render(&self) -> Vec<String> {
    let mut args = Vec::new();
    push_text(&mut args, "prefix", self.prefix.as_deref());
    push_text(&mut args, "tokenize", self.tokenize.as_deref());
    push_text(&mut args, "content", self.content.as_deref());
    push_text(&mut args, "content_rowid", self.content_rowid.as_deref());
    if let Some(columnsize) = self.columnsize {
      args.push(format!("columnsize = {columnsize}"));
    }
    push_text(&mut args, "detail", self.detail.as_deref());
    args
  }
}

fn push_text(args: &mut Vec<String>, key: &str, value: Option<&str>) {
  if let Some(value) = value {
    args.push(format!("{key} = '{}'", escape_literal(value)));
  }
}

/// SQL string-literal escaping: an embedded single quote doubles.
fn escape_literal(value: &str) -> String {
  value.replace('\'', "''")
}
