use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::{db, error::AppError};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/search", get(search))
        .route("/tags", get(get_tags))
        .route("/categories", get(get_categories))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub tags: Option<String>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub template_address: String,
    pub template_name: String,
    pub author_public_key: String,
    pub author_friendly_name: Option<String>,
    pub binary_hash: String,
    pub at_epoch: i64,
    pub metadata_hash: Option<String>,
    pub code_size: Option<i64>,
    pub is_featured: bool,
    pub has_metadata: bool,
    // Metadata fields (if available)
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
}

async fn get_tags(State(state): State<Arc<AppState>>) -> Result<Json<Vec<TagCount>>, AppError> {
    let rows = db::metadata::get_popular_tags(&state.pool, 50).await?;
    let tags = rows
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    Ok(Json(tags))
}

#[derive(Debug, Serialize)]
pub struct TagCount {
    pub tag: String,
    pub count: i64,
}

async fn get_categories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CategoryCount>>, AppError> {
    let rows = db::metadata::get_categories(&state.pool).await?;
    let categories = rows
        .into_iter()
        .map(|(category, count)| CategoryCount { category, count })
        .collect();
    Ok(Json(categories))
}

#[derive(Debug, Serialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, AppError> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let tags: Vec<String> = params
        .tags
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let rows = db::templates::search_templates(
        &state.pool,
        params.q.as_deref(),
        &tags,
        params.category.as_deref(),
        params.author.as_deref(),
        limit,
        offset,
    )
    .await?;

    let results = rows
        .into_iter()
        .map(|r| {
            let author_friendly_name = r.author_friendly_name().map(str::to_string);
            let meta_tags = r.meta_tags();
            SearchResult {
                template_address: r.template_address,
                template_name: r.template_name,
                author_public_key: r.author_public_key,
                author_friendly_name,
                binary_hash: r.binary_hash,
                at_epoch: r.at_epoch,
                metadata_hash: r.metadata_hash,
                code_size: r.code_size,
                is_featured: r.is_featured,
                has_metadata: r.meta_name.is_some(),
                meta_name: r.meta_name,
                meta_version: r.meta_version,
                meta_description: r.meta_description,
                meta_tags,
                meta_category: r.meta_category,
                meta_logo_url: r.meta_logo_url,
                meta_repository: r.meta_repository,
                meta_documentation: r.meta_documentation,
                meta_homepage: r.meta_homepage,
                meta_license: r.meta_license,
            }
        })
        .collect();

    Ok(Json(SearchResponse { results }))
}
