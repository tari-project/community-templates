use std::sync::Arc;

use super::AppState;
use crate::{
    db,
    error::{parse_template_addr, AppError},
};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/templates/featured", get(get_featured))
        .route("/templates/{addr}", get(get_template))
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub template_address: String,
    pub template_name: String,
    pub author_public_key: String,
    pub author_friendly_name: Option<String>,
    pub binary_hash: String,
    pub at_epoch: i64,
    pub metadata_hash: Option<String>,
    pub definition: Option<serde_json::Value>,
    pub code_size: Option<i64>,
    pub is_featured: bool,
    pub metadata: Option<MetadataResponse>,
}

#[derive(Debug, Serialize)]
pub struct MetadataResponse {
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
}

async fn get_featured(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TemplateResponse>>, AppError> {
    let templates = db::templates::list_featured(&state.pool).await?;
    let mut results = Vec::with_capacity(templates.len());
    for t in templates {
        let addr = db::templates::parse_addr(&t.template_address).map_err(AppError::internal)?;
        let metadata = db::metadata::get_metadata(&state.pool, &addr).await?;
        results.push(to_template_response(t, metadata));
    }
    Ok(Json(results))
}

async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(addr): Path<String>,
) -> Result<Json<TemplateResponse>, AppError> {
    let addr = parse_template_addr(&addr)?;

    let template = db::templates::get_template(&state.pool, &addr)
        .await?
        .ok_or_else(|| AppError::not_found("Template not found"))?;

    if template.is_blacklisted {
        return Err(AppError::not_found("Template not found"));
    }

    let metadata = db::metadata::get_metadata(&state.pool, &addr).await?;
    Ok(Json(to_template_response(template, metadata)))
}

fn author_friendly_name<'a>(metadata: &'a Option<db::metadata::MetadataRow>) -> Option<&'a str> {
    metadata
        .as_ref()
        .and_then(|m| m.extra.get("author_friendly_name"))
        .and_then(|v| v.as_str())
}

fn to_template_response(
    t: db::templates::TemplateRow,
    metadata: Option<db::metadata::MetadataRow>,
) -> TemplateResponse {
    let friendly_name = author_friendly_name(&metadata).map(str::to_string);
    TemplateResponse {
        template_address: t.template_address,
        template_name: t.template_name,
        author_public_key: t.author_public_key,
        author_friendly_name: friendly_name,
        binary_hash: t.binary_hash,
        at_epoch: t.at_epoch,
        metadata_hash: t.metadata_hash,
        definition: t.definition,
        code_size: t.code_size,
        is_featured: t.is_featured,
        metadata: metadata.map(|m| MetadataResponse {
            name: m.name,
            version: m.version,
            description: m.description,
            tags: m.tags,
            category: m.category,
            repository: m.repository,
            documentation: m.documentation,
            homepage: m.homepage,
            license: m.license,
            logo_url: m.logo_url,
        }),
    }
}
