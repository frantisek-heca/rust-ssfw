use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection, PgPool, Pool, Postgres};
use std::env;

pub async fn get_pool() -> Pool<Postgres> {
    // let conn = PgConnection::connect(&env::var("DATABASE_URL").unwrap())
    //     .await
    //     .unwrap(); // single connection?

    PgPool::connect(&env::var("DATABASE_URL").unwrap())
        .await
        .unwrap() // default with 10 connections

    // PgPoolOptions::new()
    //     .max_connections(50)
    //     .connect(&env::var("DATABASE_URL").unwrap())
    //     .await
    //     .unwrap()
}
