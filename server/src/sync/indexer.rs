use std::time::Duration;

use serde::Deserialize;
use sqlx::SqlitePool;
use tari_engine_types::published_template::PublishedTemplateAddress;

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

pub async fn run_sync_loop(pool: SqlitePool, config: IndexerConfig, indexer_url: String) {
    let client = reqwest::Client::new();
    let interval = Duration::from_secs(config.sync_interval_secs);

    tracing::info!(
        "Starting indexer sync loop (interval: {}s, url: {})",
        config.sync_interval_secs,
        indexer_url
    );

    loop {
        if let Err(e) = sync_once(&pool, &client, &indexer_url).await {
            tracing::error!("Indexer sync error: {e}");
        }
        tokio::time::sleep(interval).await;
    }
}

async fn sync_once(
    pool: &SqlitePool,
    client: &reqwest::Client,
    indexer_url: &str,
) -> anyhow::Result<()> {
    let since_epoch = db::templates::get_max_epoch(pool)
        .await?
        .map(|e| (e + 1) as u64);

    let mut offset = 0u64;
    let limit = 100u64;
    let mut total_new = 0usize;

    loop {
        let mut url = format!(
            "{}/templates/catalogue?limit={}&offset={}",
            indexer_url, limit, offset
        );
        if let Some(epoch) = since_epoch {
            url.push_str(&format!("&since_epoch={}", epoch));
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
        for entry in catalogue.entries {
            let addr: PublishedTemplateAddress = entry
                .template_address
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid template address from indexer: {e}"))?;

            let new_template = db::templates::NewTemplate {
                template_address: addr,
                template_name: entry.template_name,
                author_public_key: entry.author_public_key,
                binary_hash: entry.binary_hash,
                at_epoch: entry.at_epoch as i64,
                metadata_hash: entry.metadata_hash,
            };
            db::templates::upsert_template(pool, &new_template).await?;
            total_new += 1;
        }

        if (count as u64) < limit {
            break;
        }
        offset += limit;
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
    let url = format!("{}/templates/{}", indexer_url, addr.as_hash());
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "Template definition request failed with status {}",
            resp.status()
        );
    }
    let def: TemplateDefinitionResponse = resp.json().await?;
    Ok((def.definition, def.code_size as i64))
}
