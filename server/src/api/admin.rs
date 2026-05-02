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
        .route("/stats", get(get_stats))
        .route("/templates", get(list_templates))
        .route("/templates/{addr}/featured", put(set_featured))
        .route("/templates/{addr}/blacklist", put(set_blacklisted))
        .route("/admins", get(list_admins))
        .route("/admins", post(create_admin))
        .route("/admins/{id}", delete(delete_admin))
        .route("/admins/{id}/password", put(change_password))
        .route("/reindex", post(reindex))
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_templates: i64,
    pub with_metadata: i64,
    pub with_definition: i64,
    pub featured: i64,
    pub blacklisted: i64,
}

async fn get_stats(State(state): State<Arc<AppState>>) -> Result<Json<StatsResponse>, AppError> {
    let stats = db::templates::get_stats(&state.pool).await?;
    Ok(Json(StatsResponse {
        total_templates: stats.total_templates,
        with_metadata: stats.with_metadata,
        with_definition: stats.with_definition,
        featured: stats.featured,
        blacklisted: stats.blacklisted,
    }))
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
    pub logo_url: Option<String>,
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
            logo_url: t.meta_logo_url,
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

#[derive(Debug, Serialize)]
pub struct ReindexResponse {
    pub ok: bool,
    pub deleted_templates: i64,
    pub deleted_metadata: i64,
}

/// Wipe all indexed template data and the sync cursor, then wake the sync loop
/// so it re-pulls the catalogue from scratch.
///
/// Preserved across the wipe:
///   - `admins` (admin user accounts and credentials)
///   - `template_curation` (featured / blacklist flags) — these re-attach to
///     templates as they reappear from the indexer.
///
/// Serialised against the indexer sync task via `state.sync_lock` so a wipe
/// can never race with a partial write.
async fn reindex(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReindexResponse>, AppError> {
    tracing::warn!("Admin triggered database reindex: wiping templates and sync cursor");

    let _guard = state.sync_lock.lock().await;

    let mut tx = state.pool.begin().await?;

    // Order matters: template_metadata.template_address has a FK to templates.
    // SQLite doesn't enforce FKs by default in this codebase, but deleting
    // children first keeps things correct if PRAGMA foreign_keys is ever turned on.
    let metadata_deleted = sqlx::query("DELETE FROM template_metadata")
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete template_metadata: {e}");
            AppError::internal("Failed to wipe template metadata")
        })?
        .rows_affected() as i64;

    let templates_deleted = sqlx::query("DELETE FROM templates")
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete templates: {e}");
            AppError::internal("Failed to wipe templates")
        })?
        .rows_affected() as i64;

    sqlx::query("DELETE FROM sync_state")
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete sync_state: {e}");
            AppError::internal("Failed to wipe sync state")
        })?;

    tx.commit().await?;

    tracing::info!(
        "Reindex wipe complete: removed {} templates and {} metadata rows. Notifying sync loop.",
        templates_deleted,
        metadata_deleted
    );

    // Drop the lock before notifying so the loop can immediately acquire it.
    drop(_guard);
    state.reindex_notify.notify_one();

    Ok(Json(ReindexResponse {
        ok: true,
        deleted_templates: templates_deleted,
        deleted_metadata: metadata_deleted,
    }))
}
