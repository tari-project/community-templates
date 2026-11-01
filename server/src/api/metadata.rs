use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use blake2::{Blake2b512, Digest};
use serde::{Deserialize, Serialize};
use tari_crypto::{
    ristretto::{RistrettoPublicKey, RistrettoSchnorr},
    tari_utilities::ByteArray,
};
use tari_engine_types::published_template::PublishedTemplateAddress;
use tari_ootle_template_metadata::{MetadataHash, TemplateMetadata};

use super::AppState;
use crate::{
    db::{
        self,
        metadata::{FunctionDocRow, VerificationMethod},
    },
    error::{parse_template_addr, AppError},
};

/// Domain separation label for author-signed metadata updates.
const SIGNED_METADATA_DOMAIN: &[u8] = b"com.tari.ootle.community.SignedMetadataUpdate";

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/templates/{addr}/metadata", post(submit_metadata))
        .route(
            "/templates/{addr}/metadata/signed",
            post(submit_signed_metadata),
        )
}

#[derive(Debug, Serialize)]
pub struct SubmitMetadataResponse {
    pub template_address: String,
    pub verified: bool,
    pub verification_method: VerificationMethod,
    pub metadata: MetadataJson,
}

#[derive(Debug, Serialize)]
pub struct MetadataJson {
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
    pub commit_hash: Option<String>,
    pub supersedes: Option<String>,
    pub functions: Vec<FunctionDocRow>,
    pub verification_method: VerificationMethod,
}

/// Submit metadata verified against the on-chain metadata hash.
/// Body: raw CBOR-encoded TemplateMetadata.
async fn submit_metadata(
    State(state): State<Arc<AppState>>,
    Path(addr): Path<String>,
    body: Bytes,
) -> Result<Json<SubmitMetadataResponse>, AppError> {
    let addr = parse_template_addr(&addr)?;

    let template = db::templates::get_template(&state.pool, &addr)
        .await?
        .ok_or_else(|| AppError::not_found("Template not found"))?;

    if template.is_blacklisted {
        return Err(AppError::not_found("Template not found"));
    }

    let expected_hash_hex = template
        .metadata_hash
        .ok_or_else(|| AppError::bad_request("Template has no metadata hash on-chain"))?;

    let expected_hash = MetadataHash::from_hex(&expected_hash_hex)
        .map_err(|e| AppError::internal(format!("Invalid stored metadata hash: {e}")))?;

    let metadata = TemplateMetadata::from_cbor(&body)
        .map_err(|e| AppError::bad_request(format!("Invalid CBOR metadata: {e}")))?;

    let computed_hash = metadata
        .hash()
        .map_err(|e| AppError::internal(format!("Failed to hash metadata: {e}")))?;

    if computed_hash != expected_hash {
        return Err(AppError::forbidden(
            "Metadata hash does not match on-chain hash",
        ));
    }

    let resp = store_metadata(
        &state,
        &addr,
        &metadata,
        &body,
        VerificationMethod::OnChainHash,
    )
    .await?;
    Ok(Json(resp))
}

/// Signed metadata request: CBOR-encoded metadata + hex-encoded Schnorr signature.
#[derive(Debug, Deserialize)]
struct SignedMetadataRequest {
    /// CBOR-encoded TemplateMetadata (hex)
    metadata_cbor: String,
    /// Schnorr public nonce R (hex, 32 bytes)
    public_nonce: String,
    /// Schnorr signature scalar s (hex, 32 bytes)
    signature: String,
}

/// Submit metadata verified by the author's Schnorr signature.
///
/// The signature signs: Blake2b-512(domain || nonce || public_key || template_address_bytes || metadata_cbor).
/// This allows authors to update mutable metadata fields without needing to republish.
async fn submit_signed_metadata(
    State(state): State<Arc<AppState>>,
    Path(addr): Path<String>,
    Json(req): Json<SignedMetadataRequest>,
) -> Result<Json<SubmitMetadataResponse>, AppError> {
    let addr = parse_template_addr(&addr)?;

    let template = db::templates::get_template(&state.pool, &addr)
        .await?
        .ok_or_else(|| AppError::not_found("Template not found"))?;

    if template.is_blacklisted {
        return Err(AppError::not_found("Template not found"));
    }

    // Decode the metadata CBOR
    let cbor_bytes = hex::decode(&req.metadata_cbor)
        .map_err(|_| AppError::bad_request("Invalid hex in metadata_cbor"))?;
    let metadata = TemplateMetadata::from_cbor(&cbor_bytes)
        .map_err(|e| AppError::bad_request(format!("Invalid CBOR metadata: {e}")))?;

    // Validate immutable fields haven't changed (if existing metadata is present)
    if let Some(existing) = db::metadata::get_metadata(&state.pool, &addr).await? {
        if metadata.name != existing.name {
            return Err(AppError::bad_request(format!(
                "Field 'name' is immutable (was '{}', got '{}')",
                existing.name, metadata.name
            )));
        }
    }

    // Reconstruct and verify the signature
    let author_pk_bytes = hex::decode(&template.author_public_key)
        .map_err(|_| AppError::internal("Invalid stored author public key"))?;
    let author_pk = RistrettoPublicKey::from_canonical_bytes(&author_pk_bytes)
        .map_err(|_| AppError::internal("Invalid stored author public key bytes"))?;

    let nonce_bytes = hex::decode(&req.public_nonce)
        .map_err(|_| AppError::bad_request("Invalid hex in public_nonce"))?;
    let sig_bytes = hex::decode(&req.signature)
        .map_err(|_| AppError::bad_request("Invalid hex in signature"))?;

    let nonce = RistrettoPublicKey::from_canonical_bytes(&nonce_bytes)
        .map_err(|_| AppError::bad_request("Invalid public nonce"))?;
    let sig_scalar = tari_crypto::ristretto::RistrettoSecretKey::from_canonical_bytes(&sig_bytes)
        .map_err(|_| AppError::bad_request("Invalid signature scalar"))?;

    let schnorr = RistrettoSchnorr::new(nonce, sig_scalar);

    // Challenge = Blake2b-512(domain || nonce || public_key || template_address_bytes || cbor)
    let addr_bytes = addr.as_ref();
    let challenge = Blake2b512::new()
        .chain_update(SIGNED_METADATA_DOMAIN)
        .chain_update(&nonce_bytes)
        .chain_update(&author_pk_bytes)
        .chain_update(addr_bytes)
        .chain_update(&cbor_bytes)
        .finalize();

    if !schnorr.verify_raw_uniform(&author_pk, &challenge) {
        return Err(AppError::forbidden("Invalid signature"));
    }

    let resp = store_metadata(
        &state,
        &addr,
        &metadata,
        &cbor_bytes,
        VerificationMethod::AuthorSignature,
    )
    .await?;
    Ok(Json(resp))
}

/// Convert a Url to String, only allowing http/https schemes to prevent XSS via javascript: URIs.
fn safe_url_to_string(url: &url::Url) -> Option<String> {
    matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

async fn store_metadata(
    state: &AppState,
    addr: &PublishedTemplateAddress,
    metadata: &TemplateMetadata,
    cbor_bytes: &[u8],
    verification_method: VerificationMethod,
) -> Result<SubmitMetadataResponse, AppError> {
    let extra = serde_json::to_value(&metadata.extra)
        .map_err(|e| AppError::internal(format!("Failed to serialize extra metadata: {e}")))?;
    let functions: Vec<FunctionDocRow> = metadata
        .functions
        .iter()
        .map(|f| FunctionDocRow {
            name: f.name.clone(),
            doc: f.doc.clone(),
        })
        .collect();
    let new_metadata = db::metadata::NewMetadata {
        template_address: *addr,
        name: metadata.name.clone(),
        version: metadata.version.clone(),
        description: metadata.description.clone(),
        tags: metadata.tags.clone(),
        category: metadata.category.clone(),
        repository: metadata.repository.as_ref().and_then(safe_url_to_string),
        documentation: metadata.documentation.as_ref().and_then(safe_url_to_string),
        homepage: metadata.homepage.as_ref().and_then(safe_url_to_string),
        license: metadata.license.clone(),
        logo_url: metadata.logo_url.as_ref().and_then(safe_url_to_string),
        commit_hash: metadata.commit_hash.as_ref().map(|h| h.to_string()),
        supersedes: metadata.supersedes.as_ref().map(|a| a.to_string()),
        extra,
        schema_version: metadata.schema_version as i32,
        cbor_bytes: cbor_bytes.to_vec(),
        functions: functions.clone(),
        verification_method,
    };
    db::metadata::upsert_metadata(&state.pool, &new_metadata).await?;

    Ok(SubmitMetadataResponse {
        template_address: addr.to_string(),
        verified: true,
        verification_method,
        metadata: MetadataJson {
            name: metadata.name.clone(),
            version: metadata.version.clone(),
            description: metadata.description.clone(),
            tags: metadata.tags.clone(),
            category: metadata.category.clone(),
            repository: metadata.repository.as_ref().and_then(safe_url_to_string),
            documentation: metadata.documentation.as_ref().and_then(safe_url_to_string),
            homepage: metadata.homepage.as_ref().and_then(safe_url_to_string),
            license: metadata.license.clone(),
            logo_url: metadata.logo_url.as_ref().and_then(safe_url_to_string),
            commit_hash: metadata.commit_hash.as_ref().map(|h| h.to_string()),
            supersedes: metadata.supersedes.as_ref().map(|a| a.to_string()),
            functions,
            verification_method,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::templates::{canonical_addr, NewTemplate};
    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::sync::{Mutex, Notify};

    #[tokio::test]
    async fn distinguishes_on_chain_hash_and_author_signature_verification_methods() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect sqlite");
        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        let state = AppState {
            pool: pool.clone(),
            jwt_secret: vec![0x42; 32],
            sync_lock: Arc::new(Mutex::new(())),
            reindex_notify: Arc::new(Notify::new()),
        };

        let addr = PublishedTemplateAddress::from_hex(&"11".repeat(32)).expect("valid address");
        let meta = TemplateMetadata::new("amm_pool".to_string(), "1.0.0".to_string());
        let cbor_bytes = meta.to_cbor().expect("encode cbor");
        let on_chain_hash = meta.hash().expect("hash metadata").to_hex();

        db::templates::upsert_template(
            &pool,
            &NewTemplate {
                template_address: addr,
                template_name: "amm_pool".to_string(),
                author_public_key: "22".repeat(32),
                binary_hash: "33".repeat(32),
                at_epoch: 5,
                metadata_hash: Some(on_chain_hash.clone()),
            },
        )
        .await
        .expect("insert template");

        // 1. On-chain hash submission records `on_chain_hash`
        let resp = store_metadata(
            &state,
            &addr,
            &meta,
            &cbor_bytes,
            VerificationMethod::OnChainHash,
        )
        .await
        .expect("store on_chain_hash metadata");
        assert_eq!(resp.verification_method, VerificationMethod::OnChainHash);
        assert_eq!(
            resp.metadata.verification_method,
            VerificationMethod::OnChainHash
        );

        let row = db::metadata::get_metadata(&pool, &addr)
            .await
            .expect("get metadata")
            .expect("metadata row exists");
        assert_eq!(row.verification_method(), VerificationMethod::OnChainHash);

        // 2. Author signature update overwrites verification_method with `author_signature`
        let mut updated = meta.clone();
        updated.description = "Updated via author signature".to_string();
        let updated_cbor = updated.to_cbor().expect("encode updated cbor");

        let resp2 = store_metadata(
            &state,
            &addr,
            &updated,
            &updated_cbor,
            VerificationMethod::AuthorSignature,
        )
        .await
        .expect("store author_signature metadata");
        assert_eq!(
            resp2.verification_method,
            VerificationMethod::AuthorSignature
        );
        assert_eq!(
            resp2.metadata.verification_method,
            VerificationMethod::AuthorSignature
        );

        let search_rows = db::templates::search_templates(
            &pool,
            None,
            Some(&canonical_addr(&addr)),
            &[],
            None,
            None,
            10,
            0,
        )
        .await
        .expect("search templates");
        assert_eq!(search_rows.len(), 1);
        assert_eq!(
            search_rows[0].meta_verification_method(),
            Some(VerificationMethod::AuthorSignature)
        );
    }
}
