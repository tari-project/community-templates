use std::sync::Arc;

use super::AppState;
use crate::{
    api::auth,
    db,
    error::{parse_template_addr, AppError},
};
use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/templates", get(list_templates))
        .route("/templates/{addr}/featured", put(set_featured))
        .route("/templates/{addr}/blacklist", put(set_blacklisted))
        .route("/admins", get(list_admins))
        .route("/admins", post(create_admin))
        .route("/admins/{id}", delete(delete_admin))
        .route("/admins/{id}/password", put(change_password))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AdminTemplateResponse {
    pub template_address: String,
    pub template_name: String,
    pub at_epoch: i64,
    pub is_featured: bool,
    pub is_blacklisted: bool,
    pub feature_order: Option<i32>,
    pub has_definition: bool,
    pub has_metadata_hash: bool,
}

async fn list_templates(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<ListQuery>,
) -> Result<Json<Vec<AdminTemplateResponse>>, AppError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let templates = db::templates::list_all_admin(&state.pool, limit, offset).await?;
    let results = templates
        .into_iter()
        .map(|t| AdminTemplateResponse {
            template_address: t.template_address,
            template_name: t.template_name,
            at_epoch: t.at_epoch,
            is_featured: t.is_featured,
            is_blacklisted: t.is_blacklisted,
            feature_order: t.feature_order,
            has_definition: t.definition.is_some(),
            has_metadata_hash: t.metadata_hash.is_some(),
        })
        .collect();
    Ok(Json(results))
}

#[derive(Debug, Deserialize)]
pub struct SetFeaturedRequest {
    pub featured: bool,
    pub order: Option<i32>,
}

async fn set_featured(
    State(state): State<Arc<AppState>>,
    Path(addr): Path<String>,
    Json(req): Json<SetFeaturedRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let addr = parse_template_addr(&addr)?;
    let updated = db::templates::set_featured(&state.pool, &addr, req.featured, req.order).await?;
    if !updated {
        return Err(AppError::not_found("Template not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct SetBlacklistedRequest {
    pub blacklisted: bool,
}

async fn set_blacklisted(
    State(state): State<Arc<AppState>>,
    Path(addr): Path<String>,
    Json(req): Json<SetBlacklistedRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let addr = parse_template_addr(&addr)?;
    let updated = db::templates::set_blacklisted(&state.pool, &addr, req.blacklisted).await?;
    if !updated {
        return Err(AppError::not_found("Template not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Serialize)]
pub struct AdminResponse {
    pub id: i32,
    pub username: String,
    pub created_at: String,
}

async fn list_admins(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AdminResponse>>, AppError> {
    let admins = db::admins::list_admins(&state.pool).await?;
    let results = admins
        .into_iter()
        .map(|a| AdminResponse {
            id: a.id,
            username: a.username,
            created_at: a.created_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(results))
}

#[derive(Debug, Deserialize)]
pub struct CreateAdminRequest {
    pub username: String,
    pub password: String,
}

async fn create_admin(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAdminRequest>,
) -> Result<Json<AdminResponse>, AppError> {
    let hash = auth::hash_password(&req.password)?;
    let admin = db::admins::create_admin(&state.pool, &req.username, &hash).await?;
    Ok(Json(AdminResponse {
        id: admin.id,
        username: admin.username,
        created_at: admin.created_at.to_rfc3339(),
    }))
}

async fn delete_admin(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = db::admins::delete_admin(&state.pool, id).await?;
    if !deleted {
        return Err(AppError::not_found("Admin not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub new_password: String,
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let hash = auth::hash_password(&req.new_password)?;
    let updated = db::admins::update_password(&state.pool, id, &hash).await?;
    if !updated {
        return Err(AppError::not_found("Admin not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
