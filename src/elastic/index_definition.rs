#[derive(Clone)]
pub struct IndexDefinition {
    pub(crate) index_name: String,
    pub(crate) definitions_directory: String,
    pub(crate) index_prefix: String,
    pub(crate) domain_id: u8,
}

impl IndexDefinition {
    pub fn get_index_alias(&self) -> String {
        match self.index_prefix.is_empty() {
            true => format!("{}_{}", self.index_name, self.domain_id),
            false => format!(
                "{}_{}_{}",
                self.index_prefix, self.index_name, self.domain_id
            ),
        }
    }
}
