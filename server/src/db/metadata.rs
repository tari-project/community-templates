use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tari_engine_types::published_template::PublishedTemplateAddress;

use super::templates::canonical_addr;

#[derive(Debug, sqlx::FromRow)]
pub struct MetadataRow {
    #[allow(dead_code)]
    pub template_address: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub logo_url: Option<String>,
    #[allow(dead_code)]
    pub extra: serde_json::Value,
    #[allow(dead_code)]
    pub schema_version: i32,
    #[allow(dead_code)]
    pub cbor_bytes: Vec<u8>,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
}

pub struct NewMetadata {
    pub template_address: PublishedTemplateAddress,
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub logo_url: Option<String>,
    pub extra: serde_json::Value,
    pub schema_version: i32,
    pub cbor_bytes: Vec<u8>,
}

pub async fn upsert_metadata(pool: &PgPool, m: &NewMetadata) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO template_metadata (
            template_address, name, version, description, tags, category,
            repository, documentation, homepage, license, logo_url, extra,
            schema_version, cbor_bytes
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (template_address) DO UPDATE SET
            name = EXCLUDED.name,
            version = EXCLUDED.version,
            description = EXCLUDED.description,
            tags = EXCLUDED.tags,
            category = EXCLUDED.category,
            repository = EXCLUDED.repository,
            documentation = EXCLUDED.documentation,
            homepage = EXCLUDED.homepage,
            license = EXCLUDED.license,
            logo_url = EXCLUDED.logo_url,
            extra = EXCLUDED.extra,
            schema_version = EXCLUDED.schema_version,
            cbor_bytes = EXCLUDED.cbor_bytes,
            updated_at = NOW()
        "#,
    )
    .bind(canonical_addr(&m.template_address))
    .bind(&m.name)
    .bind(&m.version)
    .bind(&m.description)
    .bind(&m.tags)
    .bind(&m.category)
    .bind(&m.repository)
    .bind(&m.documentation)
    .bind(&m.homepage)
    .bind(&m.license)
    .bind(&m.logo_url)
    .bind(&m.extra)
    .bind(m.schema_version)
    .bind(&m.cbor_bytes)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_metadata(
    pool: &PgPool,
    addr: &PublishedTemplateAddress,
) -> Result<Option<MetadataRow>, sqlx::Error> {
    sqlx::query_as::<_, MetadataRow>("SELECT * FROM template_metadata WHERE template_address = $1")
        .bind(canonical_addr(addr))
        .fetch_optional(pool)
        .await
}
