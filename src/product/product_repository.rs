use crate::product::product::Product;
use sqlx::postgres::PgRow;
use sqlx::{Pool, Postgres, Row};

pub async fn get_products(pool: &Pool<Postgres>, limit: u32) -> Vec<Product> {
    sqlx::query_as::<_, Product>("SELECT * FROM products LIMIT $1")
        .bind(limit as i32)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

// pub async fn find_product_by_id(pool: &Pool<Postgres>, id: i32) -> Vec<Product> {
//     sqlx::query_as!(Product, "SELECT * FROM products WHERE id = $1", id)
//         .fetch_all(pool)
//         .await
//         .unwrap_or_default()
// }
