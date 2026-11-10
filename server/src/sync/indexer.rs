use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use sqlx::SqlitePool;
use tari_engine_types::published_template::PublishedTemplateAddress;
use tokio::sync::{Mutex, Notify};

use crate::{config::IndexerConfig, db};

/// Response from GET /templates/catalogue
#[derive(Debug, Deserialize)]
struct CatalogueResponse {
    entries: Vec<CatalogueEntry>,
}

#[derive(Debug, Deserialize)]
struct CatalogueEntry {
    template_address: String,
    template_name: String,
    author_public_key: String,
    binary_hash: String,
    at_epoch: u64,
    #[serde(default)]
    metadata_hash: Option<String>,
}

/// Response from GET /templates/{addr}
#[derive(Debug, Deserialize)]
struct TemplateDefinitionResponse {
    definition: serde_json::Value,
    code_size: usize,
}

const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn run_sync_loop(
    pool: SqlitePool,
    config: IndexerConfig,
    indexer_url: String,
    sync_lock: Arc<Mutex<()>>,
    reindex_notify: Arc<Notify>,
) {
    let client = reqwest::Client::builder()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let interval_secs = config.sync_interval_secs.max(1);
    let interval = Duration::from_secs(interval_secs);

    tracing::info!(
        "Starting indexer sync loop (interval: {}s, url: {})",
        interval_secs,
        indexer_url
    );

    loop {
        {
            // Hold the lock only for the duration of the sync pass so the admin
            // reindex handler can claim it during the sleep below.
            let _guard = sync_lock.lock().await;
            if let Err(e) = sync_once(&pool, &client, &indexer_url).await {
                tracing::error!("Indexer sync error: {e}");
            }
        }

        // Sleep until the next tick OR until a reindex notifies us, whichever
        // comes first.
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = reindex_notify.notified() => {
                tracing::info!("Reindex requested, running sync immediately");
            }
        }
    }
}

async fn sync_once(
    pool: &SqlitePool,
    client: &reqwest::Client,
    indexer_url: &str,
) -> anyhow::Result<()> {
    let mut cursor = db::sync_state::get_sync_cursor(pool).await?;
    let limit = 100u64;
    let mut total_new = 0usize;

    loop {
        let base = indexer_url.trim_end_matches('/');
        let mut url = format!("{base}/templates/catalogue?limit={limit}");
        if let Some(ref after) = cursor {
            url.push_str(&format!("&after={}", after));
        }

        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Catalogue request failed with status {}", resp.status());
        }

        let catalogue: CatalogueResponse = resp.json().await?;
        if catalogue.entries.is_empty() {
            break;
        }

        let count = catalogue.entries.len();
        for entry in &catalogue.entries {
            match parse_catalogue_entry(entry) {
                Ok(new_template) => {
                    db::templates::upsert_template(pool, &new_template).await?;
                    total_new += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Skipping invalid catalogue entry {:?}: {e}",
                        entry.template_address
                    );
                }
            }
        }

        // Persist cursor after each page for crash recovery
        let last_addr = catalogue
            .entries
            .last()
            .map(|e| e.template_address.trim())
            .unwrap_or("");
        if !should_advance_cursor(cursor.as_deref(), last_addr) {
            tracing::warn!(
                "Catalogue pagination cursor did not advance (cursor={:?}, last_addr={:?}); stopping sync pass",
                cursor,
                last_addr
            );
            break;
        }
        db::sync_state::set_sync_cursor(pool, last_addr).await?;
        cursor = Some(last_addr.to_string());

        if (count as u64) < limit {
            break;
        }
    }

    if total_new > 0 {
        tracing::info!("Synced {total_new} templates from indexer catalogue");
    }

    // Fetch definitions for templates that don't have one yet
    let missing = db::templates::get_without_definition(pool, 10).await?;
    for addr_str in missing {
        let addr: PublishedTemplateAddress = match addr_str.parse() {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Skipping invalid stored address {addr_str}: {e}");
                continue;
            }
        };
        match fetch_definition(client, indexer_url, &addr).await {
            Ok((def, code_size)) => {
                db::templates::update_template_definition(pool, &addr, &def, code_size).await?;
                tracing::debug!("Fetched definition for template {addr}");
            }
            Err(e) => {
                tracing::warn!("Failed to fetch definition for {addr}: {e}");
            }
        }
    }

    Ok(())
}

async fn fetch_definition(
    client: &reqwest::Client,
    indexer_url: &str,
    addr: &PublishedTemplateAddress,
) -> anyhow::Result<(serde_json::Value, i64)> {
    // The indexer expects the raw hex address (no prefix)
    let base = indexer_url.trim_end_matches('/');
    let url = format!("{base}/templates/{}", addr.as_hash());
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "Template definition request failed with status {}",
            resp.status()
        );
    }
    let def: TemplateDefinitionResponse = resp.json().await?;
    let code_size = i64::try_from(def.code_size)
        .map_err(|_| anyhow::anyhow!("Template code_size {} overflows i64", def.code_size))?;
    Ok((def.definition, code_size))
}

fn parse_catalogue_entry(entry: &CatalogueEntry) -> anyhow::Result<db::templates::NewTemplate> {
    let addr: PublishedTemplateAddress = entry
        .template_address
        .trim()
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid template address from indexer: {e}"))?;
    let template_name = entry.template_name.trim();
    if template_name.is_empty() {
        anyhow::bail!("Template name from indexer cannot be empty");
    }
    let at_epoch = i64::try_from(entry.at_epoch)
        .map_err(|_| anyhow::anyhow!("Template at_epoch {} overflows i64", entry.at_epoch))?;

    Ok(db::templates::NewTemplate {
        template_address: addr,
        template_name: template_name.to_string(),
        author_public_key: entry.author_public_key.trim().to_string(),
        binary_hash: entry.binary_hash.trim().to_string(),
        at_epoch,
        metadata_hash: entry
            .metadata_hash
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned),
    })
}

fn should_advance_cursor(current_cursor: Option<&str>, last_addr: &str) -> bool {
    let trimmed_last = last_addr.trim();
    if trimmed_last.is_empty() {
        return false;
    }
    match current_cursor.map(str::trim) {
        Some(prev) => prev != trimmed_last,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_addr_str() -> String {
        format!("template_{}", "ab".repeat(32))
    }

    #[test]
    fn parse_catalogue_entry_validates_fields_and_epoch_overflow() {
        let valid = CatalogueEntry {
            template_address: format!("  {}  ", valid_addr_str()),
            template_name: "  NFT Marketplace  ".to_string(),
            author_public_key: "  deadbeef  ".to_string(),
            binary_hash: "  cafebabe  ".to_string(),
            at_epoch: 42,
            metadata_hash: Some("   ".to_string()),
        };
        let parsed = parse_catalogue_entry(&valid).unwrap();
        assert_eq!(parsed.template_name, "NFT Marketplace");
        assert_eq!(parsed.author_public_key, "deadbeef");
        assert_eq!(parsed.binary_hash, "cafebabe");
        assert_eq!(parsed.at_epoch, 42);
        assert_eq!(parsed.metadata_hash, None);

        let bad_addr = CatalogueEntry {
            template_address: "not-a-valid-address".to_string(),
            template_name: "Test".to_string(),
            author_public_key: "aa".to_string(),
            binary_hash: "bb".to_string(),
            at_epoch: 1,
            metadata_hash: None,
        };
        assert!(parse_catalogue_entry(&bad_addr).is_err());

        let empty_name = CatalogueEntry {
            template_address: valid_addr_str(),
            template_name: "   ".to_string(),
            author_public_key: "aa".to_string(),
            binary_hash: "bb".to_string(),
            at_epoch: 1,
            metadata_hash: None,
        };
        assert!(parse_catalogue_entry(&empty_name).is_err());

        let overflow_epoch = CatalogueEntry {
            template_address: valid_addr_str(),
            template_name: "Test".to_string(),
            author_public_key: "aa".to_string(),
            binary_hash: "bb".to_string(),
            at_epoch: u64::MAX,
            metadata_hash: None,
        };
        assert!(parse_catalogue_entry(&overflow_epoch).is_err());
    }

    #[test]
    fn should_advance_cursor_rejects_empty_and_stationary_cursors() {
        assert!(!should_advance_cursor(None, ""));
        assert!(!should_advance_cursor(None, "   "));
        assert!(should_advance_cursor(None, "template_1"));
        assert!(should_advance_cursor(Some("template_1"), "template_2"));
        assert!(!should_advance_cursor(Some("template_1"), "template_1"));
        assert!(!should_advance_cursor(Some(" template_1 "), "template_1"));
    }
}
