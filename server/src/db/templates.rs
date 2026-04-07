use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tari_engine_types::published_template::PublishedTemplateAddress;

/// Returns the raw hex of a template address for DB storage (no `template_` prefix).
pub fn canonical_addr(addr: &PublishedTemplateAddress) -> String {
    addr.as_hash().to_string()
}

#[derive(Debug, sqlx::FromRow)]
pub struct TemplateRow {
    pub template_address: String,
    pub template_name: String,
    pub author_public_key: String,
    pub binary_hash: String,
    pub at_epoch: i64,
    pub metadata_hash: Option<String>,
    pub definition: Option<serde_json::Value>,
    pub code_size: Option<i64>,
    pub is_blacklisted: bool,
    pub is_featured: bool,
    #[allow(dead_code)]
    pub feature_order: Option<i32>,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct NewTemplate {
    pub template_address: PublishedTemplateAddress,
    pub template_name: String,
    pub author_public_key: String,
    pub binary_hash: String,
    pub at_epoch: i64,
    pub metadata_hash: Option<String>,
}

pub async fn upsert_template(pool: &SqlitePool, t: &NewTemplate) -> Result<(), sqlx::Error> {
    let addr = canonical_addr(&t.template_address);
    sqlx::query(
        r#"
        INSERT INTO templates (template_address, template_name, author_public_key, binary_hash, at_epoch, metadata_hash)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT (template_address) DO UPDATE SET
            template_name = EXCLUDED.template_name,
            author_public_key = EXCLUDED.author_public_key,
            binary_hash = EXCLUDED.binary_hash,
            at_epoch = EXCLUDED.at_epoch,
            metadata_hash = COALESCE(EXCLUDED.metadata_hash, templates.metadata_hash),
            updated_at = datetime('now')
        "#,
    )
    .bind(&addr)
    .bind(&t.template_name)
    .bind(&t.author_public_key)
    .bind(&t.binary_hash)
    .bind(t.at_epoch)
    .bind(&t.metadata_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_template_definition(
    pool: &SqlitePool,
    addr: &PublishedTemplateAddress,
    definition: &serde_json::Value,
    code_size: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE templates SET definition = ?, code_size = ?, updated_at = datetime('now')
        WHERE template_address = ?
        "#,
    )
    .bind(definition)
    .bind(code_size)
    .bind(canonical_addr(addr))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_template(
    pool: &SqlitePool,
    addr: &PublishedTemplateAddress,
) -> Result<Option<TemplateRow>, sqlx::Error> {
    sqlx::query_as::<_, TemplateRow>("SELECT * FROM templates WHERE template_address = ?")
        .bind(canonical_addr(addr))
        .fetch_optional(pool)
        .await
}

pub async fn get_max_epoch(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(Option<i64>,)> = sqlx::query_as("SELECT MAX(at_epoch) FROM templates")
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|r| r.0))
}

pub async fn list_featured_with_metadata(
    pool: &SqlitePool,
) -> Result<Vec<TemplateWithMetadataRow>, sqlx::Error> {
    sqlx::query_as::<_, TemplateWithMetadataRow>(
        r#"
        SELECT
            t.template_address, t.template_name, t.author_public_key, t.binary_hash,
            t.at_epoch, t.metadata_hash, t.definition, t.code_size,
            t.is_blacklisted, t.is_featured, t.feature_order, t.created_at, t.updated_at,
            m.name AS meta_name, m.version AS meta_version, m.description AS meta_description,
            m.tags AS meta_tags, m.category AS meta_category, m.logo_url AS meta_logo_url,
            m.repository AS meta_repository, m.documentation AS meta_documentation,
            m.homepage AS meta_homepage, m.license AS meta_license,
            m.extra AS meta_extra
        FROM templates t
        LEFT JOIN template_metadata m ON t.template_address = m.template_address
        WHERE t.is_featured = 1 AND t.is_blacklisted = 0
        ORDER BY t.feature_order ASC NULLS LAST, t.template_name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn search_templates(
    pool: &SqlitePool,
    query: Option<&str>,
    tags: &[String],
    category: Option<&str>,
    author: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<TemplateWithMetadataRow>, sqlx::Error> {
    let mut sql = String::from(
        r#"
        SELECT
            t.template_address, t.template_name, t.author_public_key, t.binary_hash,
            t.at_epoch, t.metadata_hash, t.definition, t.code_size,
            t.is_blacklisted, t.is_featured, t.feature_order, t.created_at, t.updated_at,
            m.name AS meta_name, m.version AS meta_version, m.description AS meta_description,
            m.tags AS meta_tags, m.category AS meta_category, m.logo_url AS meta_logo_url,
            m.repository AS meta_repository, m.documentation AS meta_documentation,
            m.homepage AS meta_homepage, m.license AS meta_license,
            m.extra AS meta_extra
        FROM templates t
        LEFT JOIN template_metadata m ON t.template_address = m.template_address
        WHERE t.is_blacklisted = 0
        "#,
    );

    // Collect bind values in order
    let mut binds: Vec<String> = Vec::new();

    if let Some(q) = query {
        if !q.is_empty() {
            sql.push_str(
                " AND (t.template_name LIKE '%' || ? || '%' OR COALESCE(m.description, '') LIKE '%' || ? || '%')",
            );
            binds.push(q.to_string());
            binds.push(q.to_string());
        }
    }

    for tag in tags {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM json_each(m.tags) WHERE json_each.value = ?)",
        );
        binds.push(tag.clone());
    }

    if let Some(cat) = category {
        sql.push_str(" AND m.category = ?");
        binds.push(cat.to_string());
    }

    if let Some(author_pk) = author {
        sql.push_str(" AND t.author_public_key = ?");
        binds.push(author_pk.to_string());
    }

    if query.is_some_and(|q| !q.is_empty()) {
        sql.push_str(
            " ORDER BY CASE WHEN t.template_name LIKE '%' || ? || '%' THEN 1 ELSE 2 END, t.at_epoch DESC",
        );
        binds.push(query.unwrap().to_string());
    } else {
        sql.push_str(" ORDER BY t.at_epoch DESC");
    }

    sql.push_str(" LIMIT ? OFFSET ?");

    let mut q = sqlx::query_as::<_, TemplateWithMetadataRow>(&sql);
    for val in &binds {
        q = q.bind(val);
    }
    q = q.bind(limit).bind(offset);

    q.fetch_all(pool).await
}

#[derive(Debug, sqlx::FromRow)]
pub struct TemplateWithMetadataRow {
    pub template_address: String,
    pub template_name: String,
    pub author_public_key: String,
    pub binary_hash: String,
    pub at_epoch: i64,
    pub metadata_hash: Option<String>,
    #[allow(dead_code)]
    pub definition: Option<serde_json::Value>,
    pub code_size: Option<i64>,
    #[allow(dead_code)]
    pub is_blacklisted: bool,
    pub is_featured: bool,
    #[allow(dead_code)]
    pub feature_order: Option<i32>,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
    // Joined metadata fields
    pub meta_name: Option<String>,
    pub meta_version: Option<String>,
    pub meta_description: Option<String>,
    pub meta_tags: Option<String>,
    pub meta_category: Option<String>,
    pub meta_logo_url: Option<String>,
    pub meta_repository: Option<String>,
    pub meta_documentation: Option<String>,
    pub meta_homepage: Option<String>,
    pub meta_license: Option<String>,
    pub meta_extra: Option<serde_json::Value>,
}

impl TemplateWithMetadataRow {
    /// Extract `author_friendly_name` from the extra JSON if present.
    pub fn author_friendly_name(&self) -> Option<&str> {
        self.meta_extra
            .as_ref()
            .and_then(|e| e.get("author_friendly_name"))
            .and_then(|v| v.as_str())
    }

    /// Parse meta_tags JSON string into Vec<String>.
    pub fn meta_tags(&self) -> Option<Vec<String>> {
        self.meta_tags
            .as_ref()
            .and_then(|t| serde_json::from_str(t).ok())
    }
}

// Admin queries
pub async fn set_featured(
    pool: &SqlitePool,
    addr: &PublishedTemplateAddress,
    featured: bool,
    order: Option<i32>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE templates SET is_featured = ?, feature_order = ?, updated_at = datetime('now') WHERE template_address = ?",
    )
    .bind(featured)
    .bind(order)
    .bind(canonical_addr(addr))
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_blacklisted(
    pool: &SqlitePool,
    addr: &PublishedTemplateAddress,
    blacklisted: bool,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE templates SET is_blacklisted = ?, updated_at = datetime('now') WHERE template_address = ?",
    )
    .bind(blacklisted)
    .bind(canonical_addr(addr))
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_all_admin(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<TemplateWithMetadataRow>, sqlx::Error> {
    sqlx::query_as::<_, TemplateWithMetadataRow>(
        r#"
        SELECT
            t.template_address, t.template_name, t.author_public_key, t.binary_hash,
            t.at_epoch, t.metadata_hash, t.definition, t.code_size,
            t.is_blacklisted, t.is_featured, t.feature_order, t.created_at, t.updated_at,
            m.name AS meta_name, m.version AS meta_version, m.description AS meta_description,
            m.tags AS meta_tags, m.category AS meta_category, m.logo_url AS meta_logo_url,
            m.repository AS meta_repository, m.documentation AS meta_documentation,
            m.homepage AS meta_homepage, m.license AS meta_license,
            m.extra AS meta_extra
        FROM templates t
        LEFT JOIN template_metadata m ON t.template_address = m.template_address
        ORDER BY t.at_epoch DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub struct TemplateStats {
    pub total_templates: i64,
    pub with_metadata: i64,
    pub with_definition: i64,
    pub featured: i64,
    pub blacklisted: i64,
}

pub async fn get_stats(pool: &SqlitePool) -> Result<TemplateStats, sqlx::Error> {
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            SUM(CASE WHEN definition IS NOT NULL THEN 1 ELSE 0 END),
            SUM(CASE WHEN is_featured = 1 THEN 1 ELSE 0 END),
            SUM(CASE WHEN is_blacklisted = 1 THEN 1 ELSE 0 END)
        FROM templates
        "#,
    )
    .fetch_one(pool)
    .await?;

    let (with_metadata,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM template_metadata")
        .fetch_one(pool)
        .await?;

    Ok(TemplateStats {
        total_templates: row.0,
        with_definition: row.1,
        featured: row.2,
        blacklisted: row.3,
        with_metadata,
    })
}

/// Get a batch of template addresses that still need their definition fetched.
pub async fn get_without_definition(pool: &SqlitePool, limit: i64) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT template_address FROM templates WHERE definition IS NULL AND is_blacklisted = 0 LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
