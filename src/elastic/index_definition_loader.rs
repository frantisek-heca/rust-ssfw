use crate::elastic::index_definition::IndexDefinition;

pub struct IndexDefinitionLoader {
    directory: String,
    index_prefix: String,
}

impl IndexDefinitionLoader {
    pub fn new(directory: String, index_prefix: String) -> Self {
        IndexDefinitionLoader {
            directory,
            index_prefix,
        }
    }

    pub fn get_definition(self, index_name: String, domain_id: u8) -> IndexDefinition {
        IndexDefinition {
            index_name,
            definitions_directory: self.directory,
            index_prefix: self.index_prefix,
            domain_id,
        }
    }
}
