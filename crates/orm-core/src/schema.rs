use crate::table::TableDef;

#[derive(Debug, Clone)]
pub struct SchemaRegistry {
  tables: Vec<TableDef>,
}

impl SchemaRegistry {
  pub fn from_tables(mut tables: Vec<TableDef>) -> Self {
    tables.sort_by(|a, b| a.name.cmp(&b.name));
    Self { tables }
  }

  pub fn tables(&self) -> &[TableDef] {
    &self.tables
  }

  pub fn find_table(&self, name: &str) -> Option<&TableDef> {
    self.tables.iter().find(|t| t.name == name)
  }
}
