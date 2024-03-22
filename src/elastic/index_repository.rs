use crate::elastic::error::ElasticError;
use crate::elastic::index_definition::IndexDefinition;
use crate::elastic::product_index::ProductExportData;
use elasticsearch::cat::CatIndicesParts;
use elasticsearch::indices::{IndicesExistsAliasParts, IndicesExistsParts, IndicesGetAliasParts};
use elasticsearch::{BulkOperation, BulkOperations, BulkParts, Elasticsearch};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct IndexRepository {
    pub(crate) client: Elasticsearch,
}

impl IndexRepository {
    pub async fn is_index_created(&self, index_name: &str) -> bool {
        self.client
            .indices()
            .exists(IndicesExistsParts::Index(&[index_name]))
            .send()
            .await
            .unwrap()
            .status_code()
            .is_success()
    }

    pub async fn is_alias_created(&self, index_alias: &str) -> bool {
        self.client
            .indices()
            .exists_alias(IndicesExistsAliasParts::Name(&[index_alias]))
            .send()
            .await
            .unwrap()
            .status_code()
            .is_success()
    }

    /**
     * Return all indexes that have any alias that matches index_definition
     *
     * Directly from ES API: /_alias/\*lunzo_product_1*
     */
    pub async fn get_indicies_by_index_definition(&self, index_definition: &IndexDefinition) {
        let response_body = self
            .client
            .indices()
            .get_alias(IndicesGetAliasParts::Name(&[&format!(
                "{}{}{}",
                "*",
                index_definition.get_index_alias(),
                "*"
            )]))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();

        // TODO: finish proper return type of this method and conection to the delete method
        println!("Direct: {:?}", response_body);
    }

    /**
     * This version gets all data about all indicies and then loops them and checks if correct alias
     * name does exist in the given index.
     * In opposite, the method "get_indicies_by_index_definition" does filter indicies based on alias
     * name directly in the API url.
     *
     * Why to use this 'IndicesGetAliasParts::None' version, that is obviously not using any 'filtering'
     * and then by looping through all is finding the right one??
     * If I can easily use the 'IndicesGetAliasParts::Name' in the method above.
     */
    pub async fn get_indicies_by_index_definition_raw(&self, index_definition: &IndexDefinition) {
        let indices = self.client.indices();
        let response_body = indices
            .get_alias(IndicesGetAliasParts::None)
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap();

        /*
        Sample data:
            Object {
                "emos_product_10_8125c598e02572a44ac644251e00b25d": Object {
                    "aliases": Object {
                        "emos_product_10": Object {},
                    },
                },
                "emos_product_11_fba62cdfa70e680a5449b2b7eef7bb0c": Object {
                    "aliases": Object {
                        "emos_product_11": Object {},
                    },
                }
            }
        */
        // println!("{:#?}", response_body); // for debug purposes

        /*
        json::<Value> is Enum of various possibilities. In my case, if I dump the data, it is all Objects (as presented by sample above)
        - to access these enum Value Object(s), I have to use "as_object().unwrap()" on them (repeatedly if they are nested)
        - I can access "aliases" key directly by ["aliases"] (but it will crash if the key does not exist)
        */
        for (index, aliases_object) in response_body.as_object().unwrap() {
            if aliases_object.as_object().unwrap()["aliases"]
                .as_object()
                .unwrap()
                .contains_key(&index_definition.get_index_alias())
            {
                println!(
                    "This index '{}' should be deleted, because it is associated with alias: {:#?}",
                    index,
                    index_definition.get_index_alias()
                );
            }
        }
        //
        // for (index_key, aliases_data) in response_body.as_object().unwrap() {
        //     for (alias_name, data) in aliases_data.as_object().unwrap() {
        //         println!("{:?} :: {:?}", index_key, data);
        //     }
        // }
    }

    /**
     * CAT api is mainly for plain/text results. But I am using 'format(json)' here and using it to get all indicies
     * This shouldn't be prefered method, but showing that it is possible.
     * CAT API: https://www.elastic.co/guide/en/elasticsearch/reference/7.17/cat.html
     */
    pub async fn get_all_indicies_by_cat_api_and_print_them(&self) {
        let response = self
            .client
            .cat()
            .indices(CatIndicesParts::Index(&["*"]))
            .format("json")
            .send()
            .await
            .unwrap();

        let response_body = response.json::<Value>().await.unwrap();
        for record in response_body.as_array().unwrap() {
            // print the name of each index
            println!("{}", record["index"].as_str().unwrap());
        }
    }

    /**
     * IndexRepository::findIndexNamesForAlias(string $aliasName): array
     */
    async fn find_index_names_for_alias(
        &self,
        index_alias: &str,
    ) -> Result<Vec<String>, ElasticError> {
        if !self.is_alias_created(index_alias).await {
            return Err(ElasticError::NoAlias);
        }

        // $indexesWithAlias = array_keys($this->elasticsearchClient->indices()->getAlias(['name' => $aliasName]));
        let indexes_with_alias = self
            .client
            .indices()
            .get_alias(IndicesGetAliasParts::Name(&[index_alias]))
            .send()
            .await
            .unwrap()
            .json::<Value>()
            .await
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        if indexes_with_alias.is_empty() {
            return Err(ElasticError::NoIndexFoundForAlias);
        }

        Ok(indexes_with_alias)
    }

    pub async fn bulk_update(
        &self,
        index_definition: &IndexDefinition,
        current_batch_data: &BTreeMap<i32, ProductExportData>,
    ) {
        if current_batch_data.is_empty() {
            return;
        }

        let mut ops = BulkOperations::new();
        // let mut counter = 0;
        for (id, data) in current_batch_data {
            // ops.push(BulkOperation::update(id.to_string(), data))
            //     .unwrap();

            // if counter % 1000 == 0 {
            //     println!("{}: {}", id, json!(data));
            // }

            // dbg!(data);
            ops.push(BulkOperation::update(
                id.to_string(),
                json!({
                    "doc": json!(data),
                    "doc_as_upsert": true
                }),
            ));
            // print!("{}, ", id);
            // counter += 1;
        }

        self.client
            .bulk(BulkParts::Index(&index_definition.get_index_alias()))
            .body(vec![ops])
            .send()
            .await;

        // println!("Count: {}", counter);
        // print!(".");
    }

    /**
     * IndexRepository::findCurrentIndexNameForAlias(string $aliasName): string
     */
    pub async fn find_current_index_name_for_alias(
        &self,
        index_alias: &str,
    ) -> Result<String, ElasticError> {
        let indexes_with_alias = self.find_index_names_for_alias(index_alias).await?;

        if indexes_with_alias.len() > 1 {
            return Err(ElasticError::MoreThanOneIndexFoundForAlias);
        }

        Ok(indexes_with_alias[0].clone())
    }
}
