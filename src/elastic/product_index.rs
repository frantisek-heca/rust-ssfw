use crate::product::product::ProductTranslation;
use crate::product::product::{Product, ProductDomain};
use serde::Serialize;
use sqlx::{Pool, Postgres, Row};
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct ProductExportData {
    id: i32,
    catnum: String,
    partno: String,
    ean: String,
    name: String,
    fullname: String,
    description: String,
}

#[derive(Clone)]
pub struct ProductIndex {
    pool: Pool<Postgres>,
}

pub struct ProductForElasticExport {
    pub id: i32,
    pub catnum: String,
    pub partno: Option<String>,
    pub ean: Option<String>,
}

#[derive(Default)]
pub struct ProductTranslationForElasticExport {
    name: Option<String>,
    name_prefix: Option<String>,
    name_sufix: Option<String>,
}

#[derive(Default)]
pub struct ProductDomainForElasticExport {
    domain_id: i32,
    description: Option<String>,
}

impl ProductIndex {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn get_export_data_for_batch(
        &self,
        domain_id: u8,
        last_processed_id: u32,
        batch_size: u32,
    ) -> BTreeMap<i32, ProductExportData> {
        // self.get_products_data_macro(domain_id, last_processed_id, batch_size)
        //     .await;
        // let products = self
        //     .get_products_data(domain_id, last_processed_id, batch_size)
        //     .await;

        let products = self
            .get_products_data_macro_as(domain_id, last_processed_id, batch_size)
            .await;

        let mut results: BTreeMap<i32, ProductExportData> = BTreeMap::new();
        for mut product in products {
            // musim mit "mut product" abych pozdej mohl delat nad Option hodnotama take()

            // product.with_translation(self.pool.clone()).await;
            // product.with_domain(self.pool.clone()).await;

            let product_translation = self.get_product_translation(product.id).await;
            let product_domain = self.get_product_domain(product.id).await;

            results.insert(
                product.id,
                ProductExportData {
                    id: product.id,
                    catnum: product.catnum.clone(),
                    partno: product.partno.take().unwrap_or_default(),
                    // take() vezme hodnotu z Option a nahradi za ni None
                    // tim padem neni "partno" uninitialized (neni zde move) a move semantic je pak ok
                    ean: product.ean.take().unwrap_or_default(),
                    name: product_translation
                        .name
                        .as_ref()
                        .unwrap_or(&"".to_string())
                        .clone(),
                    fullname: format!(
                        "{} {} {}",
                        product_translation.name_prefix.unwrap_or_default(),
                        product_translation.name.unwrap_or_default(),
                        product_translation.name_sufix.unwrap_or_default()
                    ),
                    description: product_domain.description.unwrap_or_default(), // nutnost použití take() bylo tímto "Error - Borrow of partially moved value: 'product'"
                },
            );
        }

        results
    }

    ///
    /// Tato verze, kde rovnou joinuju product_translations je mega pomala, cca 0.8s jeden dotaz
    /// - zpusobil to proste ten join
    /// Zatímco když jej dělám odděleně, byť pro každý produkt zvlášť, tak to frčí rychle.
    ///
    // async fn get_products_data_macro_as_sloooow(
    //     &self,
    //     domain_id: u8,
    //     last_processed_id: u32,
    //     batch_size: u32,
    // ) -> Vec<Product> {
    //     sqlx::query_as!(
    //         Product,
    //         r#"
    //         SELECT p.id, p.catnum, p.partno, p.ean, (pt.name, pt.locale) as "translation!: ProductTranslation"
    //         FROM products p
    //         INNER JOIN product_visibilities pv ON p.id = pv.product_id
    //         LEFT JOIN product_translations pt ON pt.translatable_id = p.id AND pt.locale = 'cs'
    //         WHERE pv.domain_id = $1 AND pv.visible = TRUE AND p.id > $2 AND pv.product_id > $3
    //         GROUP BY p.id, pt.name, pt.locale
    //         ORDER BY p.id
    //         LIMIT $4
    //         "#,
    //         domain_id as i32,
    //         last_processed_id as i32,
    //         last_processed_id as i32,
    //         batch_size as i32
    //     )
    //     .fetch_all(&self.pool)
    //     .await
    //     .unwrap_or_default()
    // }

    async fn get_products_data_macro_as(
        &self,
        domain_id: u8,
        last_processed_id: u32,
        batch_size: u32,
    ) -> Vec<ProductForElasticExport> {
        sqlx::query_as!(
            ProductForElasticExport,
            r#"
            SELECT p.id, p.catnum, p.partno, p.ean
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
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    // async fn get_products_data_macro(
    //     &self,
    //     domain_id: u8,
    //     last_processed_id: u32,
    //     batch_size: u32,
    // ) {
    //     sqlx::query!(
    //         r#"
    //         SELECT p.id, p.catnum, p.partno, p.ean, pt.name, pt.locale
    //         FROM products p
    //         INNER JOIN product_visibilities pv ON p.id = pv.product_id
    //         LEFT JOIN product_translations pt ON pt.translatable_id = p.id AND pt.locale = 'cs'
    //         WHERE pv.domain_id = $1 AND pv.visible = TRUE AND p.id > $2 AND pv.product_id > $3
    //         GROUP BY p.id, pt.name, pt.locale
    //         ORDER BY p.id
    //         LIMIT $4
    //         "#,
    //         domain_id as i32,
    //         last_processed_id as i32,
    //         last_processed_id as i32,
    //         batch_size as i32
    //     )
    //     .fetch_all(&self.pool)
    //     .await
    //     .unwrap_or_default();
    //
    //     products
    //     // tohle uz mi hezky vraci anonymous Struct s vysledkama, sqlx samo automaticky namapuje typy, pak uz je na me jak si to dal pouziju
    // }

    async fn get_products_data(
        &self,
        domain_id: u8,
        last_processed_id: u32,
        batch_size: u32,
    ) -> Vec<Product> {
        sqlx::query_as::<_, Product>(
            r#"SELECT p.id, p.catnum, p.partno, p.ean
            FROM products p 
            INNER JOIN product_visibilities pv ON p.id = pv.product_id  
            WHERE pv.domain_id = $1 AND pv.visible = TRUE AND p.id > $2 AND pv.product_id > $3 
            GROUP BY p.id 
            ORDER BY p.id
            LIMIT $4"#,
        )
        .bind(domain_id as i32)
        .bind(last_processed_id as i32)
        .bind(last_processed_id as i32)
        .bind(batch_size as i32)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    // async fn get_products_data_query(
    //     &self,
    //     domain_id: u8,
    //     last_processed_id: u32,
    //     batch_size: u32,
    // ) -> Vec<Product> {
    //     let rows = sqlx::query(
    //         r#"
    //         SELECT p.id, p.catnum, p.partno, p.ean, (SELECT name FROM product_translations pt WHERE pt.translatable_id = p.id AND pt.locale = 'cs') as "translation: Option<ProductTranslation>"
    //         FROM products p
    //         INNER JOIN product_visibilities pv ON p.id = pv.product_id
    //         WHERE pv.domain_id = $1 AND pv.visible = TRUE AND p.id > $2 AND pv.product_id > $3
    //         GROUP BY p.id
    //         ORDER BY p.id
    //         LIMIT $4
    //         "#,
    //     )
    //         .bind(domain_id as i32)
    //         .bind(last_processed_id as i32)
    //         .bind(last_processed_id as i32)
    //         .bind(batch_size as i32)
    //         .fetch_all(&self.pool)
    //         .await
    //         .unwrap_or_default();
    //     if rows.is_empty() {
    //         return vec![];
    //     }
    //     let mut products: Vec<Product> = Vec::with_capacity(rows.len());
    //     for row in rows {
    //         products.push(Product {
    //             id: row.get("id"),
    //             // name: row.get("name"),
    //         });
    //     }
    //     products
    // }

    pub async fn get_product_translation(
        &self,
        product_id: i32,
    ) -> ProductTranslationForElasticExport {
        sqlx::query_as!(
            ProductTranslationForElasticExport,
            r#"SELECT name, name_prefix, name_sufix
            FROM product_translations
            WHERE translatable_id = $1 AND locale = 'cs'"#,
            product_id
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_product_domain(&self, product_id: i32) -> ProductDomainForElasticExport {
        sqlx::query_as!(
            ProductDomainForElasticExport,
            r#"SELECT domain_id, description
            FROM product_domains
            WHERE product_id = $1 AND domain_id = 1"#,
            product_id
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or_default()
    }
}
