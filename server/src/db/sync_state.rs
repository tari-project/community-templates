use sqlx::SqlitePool;

const CATALOGUE_CURSOR_KEY: &str = "last_catalogue_cursor";

pub async fn get_sync_cursor(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM sync_state WHERE key = ?")
        .bind(CATALOGUE_CURSOR_KEY)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn set_sync_cursor(pool: &SqlitePool, cursor: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO sync_state (key, value) VALUES (?, ?)
        ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = datetime('now')
        "#,
    )
    .bind(CATALOGUE_CURSOR_KEY)
    .bind(cursor)
    .execute(pool)
    .await?;
    Ok(())
}
