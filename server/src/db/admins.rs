use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, sqlx::FromRow)]
pub struct AdminRow {
    pub id: i32,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

pub async fn get_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<AdminRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminRow>("SELECT * FROM admins WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn create_admin(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
) -> Result<AdminRow, sqlx::Error> {
    sqlx::query_as::<_, AdminRow>(
        "INSERT INTO admins (username, password_hash) VALUES ($1, $2) RETURNING *",
    )
    .bind(username)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

pub async fn list_admins(pool: &PgPool) -> Result<Vec<AdminRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminRow>("SELECT * FROM admins ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn delete_admin(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM admins WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn update_password(
    pool: &PgPool,
    id: i32,
    password_hash: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE admins SET password_hash = $1 WHERE id = $2")
        .bind(password_hash)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM admins")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}
