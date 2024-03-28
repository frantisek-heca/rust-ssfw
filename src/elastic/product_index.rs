use crate::elastic::discord_experiment::get_discord;
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
    short_description: String,
    brand: String,
    flags: Vec<i32>,
    categories: Vec<i32>,
}

#[derive(Clone)]
pub struct ProductIndex {
    pool: Pool<Postgres>,
}

#[derive(Debug)]
pub struct ProductForElasticExport {
    pub id: i32,
    pub catnum: String,
    pub partno: Option<String>,
    pub ean: Option<String>,
    pub brand_id: Option<i32>,
}

#[derive(Default)]
pub struct ProductTranslationForElasticExport {
    name: Option<String>,
    name_prefix: Option<String>,
    name_sufix: Option<String>,
}

#[derive(Default)]
pub struct ProductDomainForElasticExport {
    id: i32,
    domain_id: i32,
    description: Option<String>,
    short_description: Option<String>,
}

#[derive(Default)]
pub struct MainCategory {
    id: i32,
    lft: i32,
    rgt: i32,
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

            get_discord(self.pool.clone()).await;

            let product_translation = self.get_product_translation(product.id).await;
            let product_domain = self.get_product_domain(product.id).await;
            let default_pricing_group_id = 1; // todo: vytahnout ze settings
            let variants = self
                .get_sellable_variants(product.id, 1, default_pricing_group_id)
                .await;
            let product_ids = variants
                .iter()
                .map(|p| p.id)
                .chain(std::iter::once(product.id))
                .collect();
            let flag_ids = self.extract_flags_for_domain(product_ids, domain_id).await;
            let main_category = self.get_product_main_category_by_domain_id(product.id, domain_id);
            let category_ids = self.get_category_ids(product.id, domain_id).await;

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
                    short_description: product_domain.short_description.unwrap_or_default(),
                    brand: product.brand_id.map_or("".to_string(), |i| i.to_string()),
                    flags: flag_ids,
                    categories: category_ids,
                },
            );
            // dbg!(results);
            // panic!();
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
            r#"SELECT id, domain_id, description, short_description
            FROM product_domains
            WHERE product_id = $1 AND domain_id = 1"#,
            product_id
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or_default()
    }

    // V php musi udelat select * a vytahnout jen "idcka"
    // SELECT t0.akeneo_code AS akeneo_code_1, t0.id AS id_2,t0.uuid AS uuid_3, t0.rgb_color AS rgb_color_4, t0.visible AS visible_5
    // FROM flags t0
    // INNER JOIN product_domain_flags ON t0.id = product_domain_flags.flag_id
    // WHERE product_domain_flags.product_domain_id = $1
    pub async fn extract_flags_for_domain(&self, product_ids: Vec<i32>, domain_id: u8) -> Vec<i32> {
        sqlx::query!(
            r#"
            SELECT DISTINCT(pdf.flag_id)
            FROM product_domains pd
            INNER JOIN product_domain_flags pdf ON pdf.product_domain_id = pd.id
            WHERE pd.product_id = ANY ($1) AND pd.domain_id = $2;
            "#,
            &product_ids,
            domain_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .unwrap()
        .iter()
        .map(|row| row.flag_id)
        .collect()
    }

    async fn get_sellable_variants(
        &self,
        main_variant_id: i32,
        domain_id: u8,
        pricing_group_id: i32,
    ) -> Vec<ProductForElasticExport> {
        sqlx::query_as!(
            ProductForElasticExport,
            r#"SELECT p.id, p.catnum, p.partno, p.ean, p.brand_id
            FROM products p 
            INNER JOIN product_visibilities pv ON p.id = pv.product_id  
            WHERE pv.domain_id = $1
            AND pv.pricing_group_id = $2
            AND pv.visible = TRUE
            AND p.calculated_selling_denied = FALSE
            AND p.variant_type != $3
            AND p.main_variant_id = $4
            ORDER BY p.id
            "#,
            domain_id as i32,
            pricing_group_id,
            "main",
            main_variant_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_product_main_category_by_domain_id(
        &self,
        product_id: i32,
        domain_id: u8,
    ) -> MainCategory {
        sqlx::query_as!(
            MainCategory,
            r#"
            SELECT c.id, c.lft, c.rgt FROM categories c
            INNER JOIN category_domains cd ON cd.category_id = c.id
            INNER JOIN product_category_domains pcd ON (pcd.product_id = $1 AND pcd.category_id = c.id AND pcd.domain_id = $2)
            WHERE c.parent_id IS NOT NULL
            AND cd.domain_id = $3
            AND cd.visible = TRUE
            ORDER BY c.level DESC, c.lft ASC
            LIMIT 1
            "#,
            product_id,
            domain_id as i32,
            domain_id as i32
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_category_ids(&self, product_id: i32, domain_id: u8) -> Vec<i32> {
        sqlx::query!(
            r#"
            SELECT category_id 
            FROM product_category_domains 
            WHERE product_id = $1 AND domain_id = $2
            "#,
            product_id,
            domain_id as i32
        )
        .fetch_all(&self.pool)
        .await
        .unwrap()
        .iter()
        .map(|row| row.category_id)
        .collect()
    }

    // category_ids = extractCategories
    // SELECT category_id FROM product_category_domains WHERE product_id = 349625 AND domain_id = 1;

    // main_category_id = getProductMainCategoryByDomainId
    // SELECT c0_.akeneo_code                AS akeneo_code_0,
    //        c0_.svg_icon                   AS svg_icon_1,
    //        c0_.over_limit_quantity        AS over_limit_quantity_2,
    //        c0_.old_shoptet_id             AS old_shoptet_id_3,
    //        c0_.pairing_category_path      AS pairing_category_path_4,
    //        c0_.paired                     AS paired_5,
    //        c0_.exclude_from_facebook_feed AS exclude_from_facebook_feed_6,
    //        c0_.id                         AS id_7,
    //        c0_.uuid                       AS uuid_8,
    //        c0_.level                      AS level_9,
    //        c0_.lft                        AS lft_10,
    //        c0_.rgt                        AS rgt_11,
    //        c0_.google_category_id         AS google_category_id_12,
    //        c0_.redirect_category_id       AS redirect_category_id_13,
    //        c0_.zbozi_category_id          AS zbozi_category_id_14,
    //        c0_.parent_id                  AS parent_id_15
    // FROM categories c0_
    //          INNER JOIN category_domains c1_ ON (c1_.category_id = c0_.id)
    //          INNER JOIN product_category_domains p2_ ON (p2_.product_id = 349625 AND p2_.category_id = c0_.id AND p2_.domain_id = 1)
    // WHERE c0_.parent_id IS NOT NULL
    //   AND c1_.domain_id = 1
    //   AND c1_.visible = true
    // ORDER BY c0_.level DESC, c0_.lft ASC
    // LIMIT 1;

    // main_category - bude stacit id, lft, rgt
    // SELECT * FROM categories c
    // INNER JOIN category_domains cd ON cd.category_id = c.id
    // INNER JOIN product_category_domains pcd ON (pcd.product_id = 3 AND pcd.category_id = c.id AND pcd.domain_id = 1)
    // WHERE c.parent_id IS NOT NULL
    // AND cd.domain_id = 1
    // AND cd.visible = TRUE
    // ORDER BY c.level DESC, c.lft ASC
    // LIMIT 1;
}
