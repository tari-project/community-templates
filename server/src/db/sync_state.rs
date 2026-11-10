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
    let trimmed = cursor.trim();
    if trimmed.is_empty() {
        return Err(sqlx::Error::Protocol("sync cursor cannot be empty".into()));
    }
    sqlx::query(
        r#"
        INSERT INTO sync_state (key, value) VALUES (?, ?)
        ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = datetime('now')
        "#,
    )
    .bind(CATALOGUE_CURSOR_KEY)
    .bind(trimmed)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear_sync_cursor(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM sync_state WHERE key = ?")
        .bind(CATALOGUE_CURSOR_KEY)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE sync_state (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT (datetime('now')))",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn sync_cursor_roundtrip_and_rejects_empty() {
        let pool = test_pool().await;
        assert_eq!(get_sync_cursor(&pool).await.unwrap(), None);

        assert!(set_sync_cursor(&pool, "   ").await.is_err());

        set_sync_cursor(&pool, "  template_abc123  ").await.unwrap();
        assert_eq!(
            get_sync_cursor(&pool).await.unwrap(),
            Some("template_abc123".to_string())
        );

        clear_sync_cursor(&pool).await.unwrap();
        assert_eq!(get_sync_cursor(&pool).await.unwrap(), None);
    }
}
