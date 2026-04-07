use chrono::{DateTime, Utc};
use sqlx::PgPool;
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

pub async fn upsert_template(pool: &PgPool, t: &NewTemplate) -> Result<(), sqlx::Error> {
    let addr = canonical_addr(&t.template_address);
    sqlx::query(
        r#"
        INSERT INTO templates (template_address, template_name, author_public_key, binary_hash, at_epoch, metadata_hash)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (template_address) DO UPDATE SET
            template_name = EXCLUDED.template_name,
            author_public_key = EXCLUDED.author_public_key,
            binary_hash = EXCLUDED.binary_hash,
            at_epoch = EXCLUDED.at_epoch,
            metadata_hash = COALESCE(EXCLUDED.metadata_hash, templates.metadata_hash),
            updated_at = NOW()
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
    pool: &PgPool,
    addr: &PublishedTemplateAddress,
    definition: &serde_json::Value,
    code_size: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE templates SET definition = $2, code_size = $3, updated_at = NOW()
        WHERE template_address = $1
        "#,
    )
    .bind(canonical_addr(addr))
    .bind(definition)
    .bind(code_size)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_template(
    pool: &PgPool,
    addr: &PublishedTemplateAddress,
) -> Result<Option<TemplateRow>, sqlx::Error> {
    sqlx::query_as::<_, TemplateRow>("SELECT * FROM templates WHERE template_address = $1")
        .bind(canonical_addr(addr))
        .fetch_optional(pool)
        .await
}

pub async fn get_max_epoch(pool: &PgPool) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(Option<i64>,)> = sqlx::query_as("SELECT MAX(at_epoch) FROM templates")
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|r| r.0))
}

pub async fn list_featured_with_metadata(
    pool: &PgPool,
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
        WHERE t.is_featured = TRUE AND t.is_blacklisted = FALSE
        ORDER BY t.feature_order ASC NULLS LAST, t.template_name ASC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn search_templates(
    pool: &PgPool,
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
        WHERE t.is_blacklisted = FALSE
        "#,
    );

    let mut param_idx = 1u32;
    let mut conditions = Vec::new();

    if let Some(q) = query {
        if !q.is_empty() {
            let like_param = param_idx;
            let trgm_param = param_idx + 1;
            conditions.push(format!(
                "(t.template_name ILIKE ${like_param} OR COALESCE(m.description, '') ILIKE ${like_param} \
                 OR t.template_name % ${trgm_param} OR COALESCE(m.description, '') % ${trgm_param})"
            ));
            param_idx += 2;
            let _ = q;
        }
    }

    if !tags.is_empty() {
        conditions.push(format!("m.tags @> ${}", param_idx));
        param_idx += 1;
    }

    if category.is_some() {
        conditions.push(format!("m.category = ${}", param_idx));
        param_idx += 1;
    }

    if author.is_some() {
        conditions.push(format!("t.author_public_key = ${}", param_idx));
        param_idx += 1;
    }

    for cond in &conditions {
        sql.push_str(" AND ");
        sql.push_str(cond);
    }

    if query.is_some_and(|q| !q.is_empty()) {
        sql.push_str(
            " ORDER BY GREATEST(similarity(t.template_name, $2), similarity(COALESCE(m.description, ''), $2)) DESC"
        );
    } else {
        sql.push_str(" ORDER BY t.at_epoch DESC");
    }

    sql.push_str(&format!(" LIMIT ${} OFFSET ${}", param_idx, param_idx + 1));

    let mut q = sqlx::query_as::<_, TemplateWithMetadataRow>(&sql);

    if let Some(query_str) = query {
        if !query_str.is_empty() {
            q = q.bind(format!("%{query_str}%")); // ILIKE pattern
            q = q.bind(query_str); // trigram similarity
        }
    }
    if !tags.is_empty() {
        q = q.bind(tags);
    }
    if let Some(cat) = category {
        q = q.bind(cat);
    }
    if let Some(author_pk) = author {
        q = q.bind(author_pk);
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
    pub meta_tags: Option<Vec<String>>,
    pub meta_category: Option<String>,
    pub meta_logo_url: Option<String>,
    pub meta_repository: Option<String>,
    pub meta_documentation: Option<String>,
    pub meta_homepage: Option<String>,
    pub meta_license: Option<String>,
    pub meta_extra: Option<serde_json::Value>,
}

impl TemplateWithMetadataRow {
    /// Extract `author_friendly_name` from the extra JSONB if present.
    pub fn author_friendly_name(&self) -> Option<&str> {
        self.meta_extra
            .as_ref()
            .and_then(|e| e.get("author_friendly_name"))
            .and_then(|v| v.as_str())
    }
}

// Admin queries
pub async fn set_featured(
    pool: &PgPool,
    addr: &PublishedTemplateAddress,
    featured: bool,
    order: Option<i32>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE templates SET is_featured = $2, feature_order = $3, updated_at = NOW() WHERE template_address = $1",
    )
    .bind(canonical_addr(addr))
    .bind(featured)
    .bind(order)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_blacklisted(
    pool: &PgPool,
    addr: &PublishedTemplateAddress,
    blacklisted: bool,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE templates SET is_blacklisted = $2, updated_at = NOW() WHERE template_address = $1",
    )
    .bind(canonical_addr(addr))
    .bind(blacklisted)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_all_admin(
    pool: &PgPool,
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
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn get_stats(pool: &PgPool) -> Result<crate::api::admin::StatsResponse, sqlx::Error> {
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*),
            COUNT(*) FILTER (WHERE definition IS NOT NULL),
            COUNT(*) FILTER (WHERE is_featured = TRUE),
            COUNT(*) FILTER (WHERE is_blacklisted = TRUE)
        FROM templates
        "#,
    )
    .fetch_one(pool)
    .await?;

    let (with_metadata,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM template_metadata")
        .fetch_one(pool)
        .await?;

    Ok(crate::api::admin::StatsResponse {
        total_templates: row.0,
        with_definition: row.1,
        featured: row.2,
        blacklisted: row.3,
        with_metadata,
    })
}

/// Get a batch of template addresses that still need their definition fetched.
pub async fn get_without_definition(pool: &PgPool, limit: i64) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT template_address FROM templates WHERE definition IS NULL AND is_blacklisted = FALSE LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}
