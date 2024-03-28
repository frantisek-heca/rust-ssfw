#![allow(dead_code, unused)]

mod elastic;
mod postgres;
mod product;
mod utils;

use crate::elastic::index_facade::IndexFacade;
use crate::elastic::index_repository::IndexRepository;
use crate::elastic::product_index::{
    ProductDomainForElasticExport, ProductForElasticExport, ProductIndex,
};
use crate::postgres::postgres_connect;
use crate::product::product_repository;
use dotenvy::dotenv;
use elasticsearch::cat::CatIndicesParts;
use elasticsearch::indices::{
    IndicesCreateParts, IndicesExistsParts, IndicesGetAliasParts, IndicesPutAliasParts,
};
use elasticsearch::Elasticsearch;
use serde_json::Value;
use sqlx::{Connection, FromRow, Pool, Postgres, Row};
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

use askama::Template;
use axum::{
    extract,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenv().expect(".env file not found");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_templates=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // build our application with some routes
    let app = Router::new()
        .route("/greet/:name", get(greet))
        .nest_service("/assets", tower_http::services::ServeDir::new("assets"));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn greet(extract::Path(name): extract::Path<String>) -> impl IntoResponse {
    let pool = postgres::postgres_connect::get_pool().await;
    let products = get_products(pool.clone(), 1, 1, 10000).await;
    let mut products_for_template = vec![];
    for product in products {
        let product_id = product.id;
        products_for_template.push(ProductForTemplate {
            product,
            description: get_product_domain(pool.clone(), product_id)
                .await
                .description,
        })
    }
    let template = HelloTemplate {
        products: products_for_template,
    };
    HtmlTemplate(template)
}

struct ProductForTemplate {
    product: ProductForElasticExport,
    description: Option<String>,
}

#[derive(Template)]
#[template(path = "hello.askama.html")]
struct HelloTemplate {
    products: Vec<ProductForTemplate>,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}

async fn get_products(
    pool: Pool<Postgres>,
    domain_id: u8,
    last_processed_id: u32,
    batch_size: u32,
) -> Vec<ProductForElasticExport> {
    sqlx::query_as!(
        ProductForElasticExport,
        r#"
            SELECT p.id, p.catnum, p.partno, p.ean, p.brand_id
            FROM products p
            INNER JOIN product_visibilities pv ON p.id = pv.product_id
            WHERE pv.domain_id = $1 AND pv.visible = TRUE AND p.id > $2 AND pv.product_id > $3
            GROUP BY p.id
            ORDER BY p.id
            LIMIT $4
            "#,
        domain_id as i32,
        last_processed_id as i32,
        last_processed_id as i32,
        batch_size as i32
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default()
}

pub async fn get_product_domain(
    pool: Pool<Postgres>,
    product_id: i32,
) -> ProductDomainForElasticExport {
    sqlx::query_as!(
        ProductDomainForElasticExport,
        r#"SELECT id, domain_id, description, short_description
            FROM product_domains
            WHERE product_id = $1 AND domain_id = 1"#,
        product_id
    )
    .fetch_one(&pool)
    .await
    .unwrap_or_default()
}

#[tokio::main]
async fn main2() -> Result<(), Box<dyn std::error::Error>> {
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
