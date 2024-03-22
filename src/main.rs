#![allow(dead_code, unused)]

mod elastic;
mod postgres;
mod product;
mod utils;

use crate::elastic::index_facade::IndexFacade;
use crate::elastic::index_repository::IndexRepository;
use crate::elastic::product_index::ProductIndex;
use crate::postgres::postgres_connect;
use crate::product::product_repository;
use dotenvy::dotenv;
use elasticsearch::cat::CatIndicesParts;
use elasticsearch::indices::{
    IndicesCreateParts, IndicesExistsParts, IndicesGetAliasParts, IndicesPutAliasParts,
};
use elasticsearch::Elasticsearch;
use serde_json::Value;
use sqlx::{Connection, FromRow, Row};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use tokio::fs;

/*#[derive(Debug, FromRow)]
struct SettingValue {
    domain_id: i32,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // let conn = PgConnection::connect(&env::var("DATABASE_URL").unwrap()).await?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    for _x in 1..100 {
        let select_query = sqlx::query("SELECT domain_id, name FROM setting_values");
        let _setting_values = select_query
            .map(|row: PgRow| SettingValue {
                domain_id: row.get("domain_id"),
                name: row.get("name"),
            })
            .fetch_all(&pool)
            .await?;
        print!(".");
        // println!("\n==== setting_values: \n{:#?}", setting_values);
    }

    Ok(())
}*/
/*
#[derive(Debug)]
struct SettingValue {
    domain_id: i32,
    name: String,
}

fn main() {
    let mut client =
        Client::connect(&env::var("DATABASE_URL").unwrap(), NoTls).unwrap();

    for _x in 1..100 {
        let mut setting_values = vec![];
        for row in client
            .query("SELECT domain_id, name FROM setting_values", &[])
            .unwrap()
        {
            setting_values.push(SettingValue {
                domain_id: row.get(0),
                name: row.get(1),
            });
        }
        // println!("\n==== setting_values: \n{:#?}", setting_values);
        print!(".");
    }
}
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().expect(".env file not found");
    let shopsys_elasticsearch_structure_dir = "./src/resources/definition/";

    let pool = postgres::postgres_connect::get_pool().await;
    // let products = product_repository::get_products(&pool, 2).await;
    // println!("\n==== products: \n{:#?}", products);

    // ---------------------------------------------------------------------------------------------
    // ELASTICSEARCH PART
    // ---------------------------------------------------------------------------------------------
    let index_definition_loader = elastic::index_definition_loader::IndexDefinitionLoader::new(
        shopsys_elasticsearch_structure_dir.to_string(),
        env::var("ELASTIC_SEARCH_INDEX_PREFIX").unwrap(),
    );

    // product definition
    let index_definition = index_definition_loader.get_definition("product".to_string(), 1);

    let index_repository = IndexRepository {
        client: Elasticsearch::default(),
    };

    let product_index = ProductIndex::new(pool.clone());
    let index_facade = IndexFacade::new(&index_repository);

    index_facade.export(&product_index, &index_definition).await;
    // index_facade.migrate(&index_definition).await;

    return Ok(());

    // index_repository
    //     .get_all_indicies_by_cat_api_and_print_them()
    //     .await;
    // return Ok(());

    // ---------------------------------------------------------------------------------------------
    // implementation of IndexRepository::deleteIndexByIndexDefinition 5.2.23
    // ---------------------------------------------------------------------------------------------

    println!(
        "\n\nSearching for alias: {}",
        index_definition.get_index_alias()
    );

    index_repository
        .get_indicies_by_index_definition_raw(&index_definition)
        .await;

    println!("\n\n");

    index_repository
        .get_indicies_by_index_definition(&index_definition)
        .await;

    // panic!();

    // TODO: refactor getAlias inside of delete_index_by... into standalone method and:
    //   [x] try to get aliases directly from ES API with /_alias/{lunzo_product_1}
    //      - done "get_indicies_by_index_definition"
    //   [] or maybe using the /_cat/_aliases/ endpoint

    // ---------------------------------------------------------------------------------------------

    let index_name = "test_1_5e43a79979286148d59dee146ced642e";
    println!(
        "found index '{}': {:?}",
        index_name,
        index_repository.is_index_created(index_name).await
    );

    println!(
        "found index alias '{}': {:?}",
        index_definition.get_index_alias(),
        index_repository
            .is_alias_created(&index_definition.get_index_alias())
            .await
    );

    if !index_repository.is_index_created(index_name).await {
        // create index
        let path = "./src/1.json";
        let data_definition = fs::read_to_string(path).await?;
        let json: Value = serde_json::from_str(&data_definition)?;
        let res = index_repository
            .client
            .indices()
            .create(IndicesCreateParts::Index(index_name))
            .body(json)
            .send()
            .await?;
        println!("created: {:?}", res.status_code());
        println!("{:#?}", res.json::<Value>().await?);

        // create alias
        let res = index_repository
            .client
            .indices()
            .put_alias(IndicesPutAliasParts::IndexName(&[index_name], "test_1"))
            .send()
            .await?;
        println!("{:#?}", res.json::<Value>().await?);
    }

    let response = index_repository
        .client
        .indices()
        .get_alias(IndicesGetAliasParts::Name(&[
            &index_definition.get_index_alias()
        ]))
        .send()
        .await?;

    println!("{:?}", response.json::<Value>().await.unwrap());

    Ok(())
}
