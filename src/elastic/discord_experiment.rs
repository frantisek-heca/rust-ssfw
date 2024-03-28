use sqlx::types::Uuid;
use sqlx::{Pool, Postgres, Type};
use time::PrimitiveDateTime;

#[derive(Debug, Type)]
struct ParameterTemplate {
    id: i32,
    name: String,
    created_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Type)]
struct Parameter {
    id: i32,
    uid: Uuid,
}

#[derive(Debug)]
struct ParameterTemplateWithParameters {
    parameter_template: ParameterTemplate,
    parameters: Vec<Parameter>,
}

pub async fn get_discord(pool: Pool<Postgres>) {
    let result = sqlx::query_as!(
            ParameterTemplateWithParameters,
            r#"
            SELECT (pt.id, pt.name, pt.created_at) as "parameter_template!: ParameterTemplate", ARRAY_AGG((p.id, p.uuid)) as "parameters!: Vec<Parameter>"
            FROM parameter_templates pt
            JOIN parameter_templates_parameters ptp ON ptp.parameter_template_id = pt.id
            JOIN parameters p ON ptp.parameter_id = p.id
            WHERE pt.id = 30
            GROUP BY pt.id;
            "#
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
    dbg!(result);
    panic!();
}
