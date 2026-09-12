use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// One column in an index, optionally descending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexColumn {
  pub name: String,
  pub desc: bool,
}

impl IndexColumn {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      desc: false,
    }
  }

  pub fn desc(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      desc: true,
    }
  }
}

impl From<&str> for IndexColumn {
  fn from(name: &str) -> Self {
    Self::new(name)
  }
}

impl From<String> for IndexColumn {
  fn from(name: String) -> Self {
    Self::new(name)
  }
}

impl Serialize for IndexColumn {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    #[derive(Serialize)]
    struct Wire<'a> {
      name: &'a str,
      #[serde(skip_serializing_if = "std::ops::Not::not")]
      desc: bool,
    }
    Wire {
      name: &self.name,
      desc: self.desc,
    }
    .serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for IndexColumn {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Wire {
      Name(String),
      Full {
        name: String,
        #[serde(default)]
        desc: bool,
      },
    }
    match Wire::deserialize(deserializer)? {
      Wire::Name(name) => Ok(Self { name, desc: false }),
      Wire::Full { name, desc } => Ok(Self { name, desc }),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexDef {
  pub name: String,
  pub columns: Vec<IndexColumn>,
  pub unique: bool,
  /// Partial-index predicate, rendered as `WHERE <predicate>` when set.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub where_clause: Option<String>,
}
