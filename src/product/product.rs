use serde::Serialize;
use sqlx::{Pool, Postgres};

#[derive(Default, Debug, sqlx::FromRow)]
#[sqlx(default)]
pub struct Product {
    pub id: i32,
    pub catnum: String,
    pub partno: Option<String>,
    pub ean: Option<String>,
    pub translation: Option<ProductTranslation>,
    pub domain: Option<ProductDomain>,
}

#[derive(Default, Debug, sqlx::FromRow, Serialize, sqlx::Type)]
pub struct ProductTranslation {
    name: Option<String>,
    name_prefix: Option<String>,
    name_sufix: Option<String>,
}

#[derive(Default, Debug, sqlx::FromRow, Serialize, sqlx::Type)]
pub struct ProductDomain {
    domain_id: i32,
    description: Option<String>,
}

impl Product {
    pub async fn with_translation(&mut self, pool: Pool<Postgres>) {
        if self.translation.is_some() {
            return;
        }

        let translation = sqlx::query_as!(
            ProductTranslation,
            r#"SELECT name, name_prefix, name_sufix
            FROM product_translations
            WHERE translatable_id = $1 AND locale = 'cs'"#,
            self.id
        )
        .fetch_one(&pool)
        .await
        .unwrap_or_default();

        self.translation = Some(translation);
    }

    pub async fn with_domain(&mut self, pool: Pool<Postgres>) {
        if self.domain.is_some() {
            return;
        }

        let domain = sqlx::query_as!(
            ProductDomain,
            r#"SELECT domain_id, description
            FROM product_domains
            WHERE product_id = $1 AND domain_id = 1"#,
            self.id
        )
        .fetch_one(&pool)
        .await
        .unwrap_or_default();

        self.domain = Some(domain);
    }

    pub fn name(&self) -> String {
        if self.translation.is_none() {
            return "".to_string();
        }

        self.translation
            .as_ref()
            .unwrap()
            .name
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone()
    }

    pub fn name_prefix(&self) -> String {
        if self.translation.is_none() {
            return "".to_string();
        }

        self.translation
            .as_ref()
            .unwrap()
            .name_prefix
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone()
    }

    pub fn name_sufix(&self) -> String {
        if self.translation.is_none() {
            return "".to_string();
        }

        self.translation
            .as_ref()
            .unwrap()
            .name_sufix
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone()
    }

    pub fn fullname(&self) -> String {
        format!(
            "{} {} {}",
            self.name_prefix(),
            self.name(),
            self.name_sufix()
        )
    }

    pub fn description(&self) -> String {
        if self.domain.is_none() {
            return "".to_string();
        }

        self.domain
            .as_ref()
            .unwrap()
            .description
            .as_ref()
            .unwrap_or(&"".to_string())
            .clone()
    }
}
