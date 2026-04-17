use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tari_engine_types::published_template::PublishedTemplateAddress;

use super::templates::canonical_addr;

#[derive(Debug, sqlx::FromRow)]
pub struct MetadataRow {
    #[allow(dead_code)]
    pub template_address: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: String,
    pub category: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub logo_url: Option<String>,
    pub commit_hash: Option<String>,
    pub supersedes: Option<String>,
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

impl MetadataRow {
    pub fn tags(&self) -> Vec<String> {
        serde_json::from_str(&self.tags).unwrap_or_default()
    }
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
    pub commit_hash: Option<String>,
    pub supersedes: Option<String>,
    pub extra: serde_json::Value,
    pub schema_version: i32,
    pub cbor_bytes: Vec<u8>,
}

pub async fn upsert_metadata(pool: &SqlitePool, m: &NewMetadata) -> Result<(), sqlx::Error> {
    let tags_json = serde_json::to_string(&m.tags).unwrap_or_else(|_| "[]".to_string());
    sqlx::query(
        r#"
        INSERT INTO template_metadata (
            template_address, name, version, description, tags, category,
            repository, documentation, homepage, license, logo_url,
            commit_hash, supersedes, extra, schema_version, cbor_bytes
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            commit_hash = EXCLUDED.commit_hash,
            supersedes = EXCLUDED.supersedes,
            extra = EXCLUDED.extra,
            schema_version = EXCLUDED.schema_version,
            cbor_bytes = EXCLUDED.cbor_bytes,
            updated_at = datetime('now')
        "#,
    )
    .bind(canonical_addr(&m.template_address))
    .bind(&m.name)
    .bind(&m.version)
    .bind(&m.description)
    .bind(&tags_json)
    .bind(&m.category)
    .bind(&m.repository)
    .bind(&m.documentation)
    .bind(&m.homepage)
    .bind(&m.license)
    .bind(&m.logo_url)
    .bind(&m.commit_hash)
    .bind(&m.supersedes)
    .bind(&m.extra)
    .bind(m.schema_version)
    .bind(&m.cbor_bytes)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_metadata(
    pool: &SqlitePool,
    addr: &PublishedTemplateAddress,
) -> Result<Option<MetadataRow>, sqlx::Error> {
    sqlx::query_as::<_, MetadataRow>("SELECT * FROM template_metadata WHERE template_address = ?")
        .bind(canonical_addr(addr))
        .fetch_optional(pool)
        .await
}

/// Returns all categories with their template counts, ordered by frequency.
pub async fn get_categories(pool: &SqlitePool) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT category, COUNT(*) AS cnt
        FROM template_metadata
        WHERE category IS NOT NULL AND category != ''
        GROUP BY category
        ORDER BY cnt DESC
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Returns the most popular tags across all templates, ordered by frequency.
pub async fn get_popular_tags(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT json_each.value AS tag, COUNT(*) AS cnt
        FROM template_metadata, json_each(template_metadata.tags)
        GROUP BY json_each.value
        ORDER BY cnt DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
