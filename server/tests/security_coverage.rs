use std::sync::Arc;

use blake2::{Blake2b512, Digest};
use jsonwebtoken::{encode, EncodingKey, Header};
use ootle_community_templates::{
    api::{self, auth::Claims, AppState},
    db::{
        self,
        templates::{canonical_addr, NewTemplate},
    },
};
use reqwest::StatusCode;
use serde_json::{json, Value};
use sqlx::sqlite::SqlitePoolOptions;
use tari_crypto::{
    keys::{PublicKey, SecretKey},
    ristretto::{RistrettoPublicKey, RistrettoSchnorr, RistrettoSecretKey},
    tari_utilities::ByteArray,
};
use tari_engine_types::published_template::PublishedTemplateAddress;
use tari_ootle_template_metadata::TemplateMetadata;
use tokio::sync::{Mutex, Notify};

const SIGNED_METADATA_DOMAIN: &[u8] = b"com.tari.ootle.community.SignedMetadataUpdate";
const TEST_JWT_SECRET: &[u8] = b"test-jwt-secret-with-sufficient-entropy-32b";

struct TestApp {
    base_url: String,
    client: reqwest::Client,
    pool: sqlx::SqlitePool,
    jwt_secret: Vec<u8>,
}

impl TestApp {
    async fn spawn() -> Self {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");

        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        let jwt_secret = TEST_JWT_SECRET.to_vec();
        let state = Arc::new(AppState {
            pool: pool.clone(),
            jwt_secret: jwt_secret.clone(),
            sync_lock: Arc::new(Mutex::new(())),
            reindex_notify: Arc::new(Notify::new()),
        });

        let app = api::router(state, "/");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test app");
        });

        Self {
            base_url: format!("http://{addr}"),
            client: reqwest::Client::new(),
            pool,
            jwt_secret,
        }
    }
}

fn make_keypair(seed: u8) -> (RistrettoSecretKey, RistrettoPublicKey) {
    let mut bytes = [0u8; 64];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = seed.wrapping_add(i as u8).wrapping_mul(17);
    }
    let sk = RistrettoSecretKey::from_uniform_bytes(&bytes).expect("secret key from uniform bytes");
    let pk = RistrettoPublicKey::from_secret_key(&sk);
    (sk, pk)
}

fn make_template_address(fill: u8) -> PublishedTemplateAddress {
    PublishedTemplateAddress::from_hex(&format!("{fill:02x}").repeat(32))
        .expect("valid template address")
}

fn sample_metadata(name: &str, description: &str) -> TemplateMetadata {
    let mut meta = TemplateMetadata::new(name.to_string(), "1.0.0".to_string());
    meta.description = description.to_string();
    meta.category = Some("defi".to_string());
    meta.tags = vec!["dex".to_string(), "amm".to_string()];
    meta
}

fn sign_metadata_payload(
    author_sk: &RistrettoSecretKey,
    author_pk: &RistrettoPublicKey,
    nonce_seed: u8,
    addr: &PublishedTemplateAddress,
    cbor_bytes: &[u8],
) -> (Vec<u8>, Vec<u8>) {
    let (nonce_sk, public_nonce) = make_keypair(nonce_seed);
    let nonce_bytes = public_nonce.as_bytes().to_vec();

    let challenge = Blake2b512::new()
        .chain_update(SIGNED_METADATA_DOMAIN)
        .chain_update(&nonce_bytes)
        .chain_update(author_pk.as_bytes())
        .chain_update(addr.as_ref())
        .chain_update(cbor_bytes)
        .finalize();

    let schnorr = RistrettoSchnorr::sign_raw_uniform(author_sk, nonce_sk, &challenge)
        .expect("sign_raw_uniform");
    let sig_bytes = schnorr.get_signature().as_bytes().to_vec();
    (nonce_bytes, sig_bytes)
}

#[tokio::test]
async fn signed_metadata_accepts_valid_and_rejects_flipped_signature_nonce_cbor_or_address() {
    let app = TestApp::spawn().await;
    let (author_sk, author_pk) = make_keypair(1);
    let addr1 = make_template_address(0x11);
    let addr2 = make_template_address(0x12);

    for (addr, name) in [(&addr1, "swap_pool"), (&addr2, "swap_pool_v2")] {
        db::templates::upsert_template(
            &app.pool,
            &NewTemplate {
                template_address: *addr,
                template_name: name.to_string(),
                author_public_key: hex::encode(author_pk.as_bytes()),
                binary_hash: "aa".repeat(32),
                at_epoch: 10,
                metadata_hash: None,
            },
        )
        .await
        .expect("insert template");
    }

    let meta = sample_metadata("swap_pool", "Initial signed description");
    let cbor_bytes = meta.to_cbor().expect("encode cbor");
    let (nonce_bytes, sig_bytes) =
        sign_metadata_payload(&author_sk, &author_pk, 42, &addr1, &cbor_bytes);

    // 1a. Flipped bit in signature scalar -> rejected with 403 Forbidden
    let mut bad_sig = sig_bytes.clone();
    bad_sig[0] ^= 0x01;
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata/signed",
            app.base_url,
            canonical_addr(&addr1)
        ))
        .json(&json!({
            "metadata_cbor": hex::encode(&cbor_bytes),
            "public_nonce": hex::encode(&nonce_bytes),
            "signature": hex::encode(&bad_sig),
        }))
        .send()
        .await
        .expect("send bad sig request");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 1b. Flipped bit in public nonce -> rejected (400 if non-canonical point, 403 if wrong valid point)
    let mut bad_nonce = nonce_bytes.clone();
    bad_nonce[0] ^= 0x01;
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata/signed",
            app.base_url,
            canonical_addr(&addr1)
        ))
        .json(&json!({
            "metadata_cbor": hex::encode(&cbor_bytes),
            "public_nonce": hex::encode(&bad_nonce),
            "signature": hex::encode(&sig_bytes),
        }))
        .send()
        .await
        .expect("send flipped nonce request");
    assert!(
        resp.status() == StatusCode::BAD_REQUEST || resp.status() == StatusCode::FORBIDDEN,
        "expected 400 or 403 for flipped nonce, got {}",
        resp.status()
    );

    // Also test a valid canonical Ristretto point that differs from the signed nonce -> 403 Forbidden
    let (_, other_nonce_pk) = make_keypair(99);
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata/signed",
            app.base_url,
            canonical_addr(&addr1)
        ))
        .json(&json!({
            "metadata_cbor": hex::encode(&cbor_bytes),
            "public_nonce": hex::encode(other_nonce_pk.as_bytes()),
            "signature": hex::encode(&sig_bytes),
        }))
        .send()
        .await
        .expect("send wrong canonical nonce request");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 1c. Flipped bit in CBOR body -> rejected (400 if invalid CBOR, 403 if valid CBOR with wrong hash)
    let mut bad_cbor = cbor_bytes.clone();
    let desc_pos = bad_cbor
        .windows(b"Initial".len())
        .position(|w| w == b"Initial")
        .expect("find description substring in CBOR");
    bad_cbor[desc_pos] ^= 0x01;
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata/signed",
            app.base_url,
            canonical_addr(&addr1)
        ))
        .json(&json!({
            "metadata_cbor": hex::encode(&bad_cbor),
            "public_nonce": hex::encode(&nonce_bytes),
            "signature": hex::encode(&sig_bytes),
        }))
        .send()
        .await
        .expect("send flipped cbor request");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 1d. Same valid signature submitted against a different template address (flipped bit in address) -> 403 Forbidden
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata/signed",
            app.base_url,
            canonical_addr(&addr2)
        ))
        .json(&json!({
            "metadata_cbor": hex::encode(&cbor_bytes),
            "public_nonce": hex::encode(&nonce_bytes),
            "signature": hex::encode(&sig_bytes),
        }))
        .send()
        .await
        .expect("send wrong template address request");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 1e. Valid signed submission is accepted with 200 OK
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata/signed",
            app.base_url,
            canonical_addr(&addr1)
        ))
        .json(&json!({
            "metadata_cbor": hex::encode(&cbor_bytes),
            "public_nonce": hex::encode(&nonce_bytes),
            "signature": hex::encode(&sig_bytes),
        }))
        .send()
        .await
        .expect("send valid signed metadata");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("parse response json");
    assert_eq!(body["verified"], json!(true));
    assert_eq!(body["metadata"]["name"], json!("swap_pool"));
    assert_eq!(
        body["metadata"]["description"],
        json!("Initial signed description")
    );
}

#[tokio::test]
async fn on_chain_hash_submission_rejects_mismatch_with_403_and_accepts_matching_cbor() {
    let app = TestApp::spawn().await;
    let (_, author_pk) = make_keypair(2);
    let addr = make_template_address(0x22);

    let expected_meta = sample_metadata("token_vault", "On-chain anchored description");
    let expected_cbor = expected_meta.to_cbor().expect("encode expected cbor");
    let expected_hash = expected_meta.hash().expect("hash metadata").to_hex();

    db::templates::upsert_template(
        &app.pool,
        &NewTemplate {
            template_address: addr,
            template_name: "token_vault".to_string(),
            author_public_key: hex::encode(author_pk.as_bytes()),
            binary_hash: "bb".repeat(32),
            at_epoch: 20,
            metadata_hash: Some(expected_hash),
        },
    )
    .await
    .expect("insert template");

    // Tampered CBOR body whose hash does not match on-chain metadata_hash -> 403 Forbidden
    let tampered_meta = sample_metadata("token_vault", "Tampered off-chain description");
    let tampered_cbor = tampered_meta.to_cbor().expect("encode tampered cbor");
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata",
            app.base_url,
            canonical_addr(&addr)
        ))
        .body(tampered_cbor)
        .send()
        .await
        .expect("send mismatched cbor");
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // Matching CBOR body -> 200 OK
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{}/metadata",
            app.base_url,
            canonical_addr(&addr)
        ))
        .body(expected_cbor)
        .send()
        .await
        .expect("send matching cbor");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.expect("parse response json");
    assert_eq!(body["verified"], json!(true));
    assert_eq!(body["metadata"]["name"], json!("token_vault"));
}

#[tokio::test]
async fn blacklisted_template_is_filtered_from_public_endpoints_and_submissions() {
    let app = TestApp::spawn().await;
    let (author_sk, author_pk) = make_keypair(3);
    let visible_addr = make_template_address(0x31);
    let blacklisted_addr = make_template_address(0x32);

    let visible_meta = sample_metadata("visible_template", "Visible template description");
    let visible_cbor = visible_meta.to_cbor().expect("visible cbor");
    let visible_hash = visible_meta.hash().expect("visible hash").to_hex();

    let mut blk_meta =
        TemplateMetadata::new("blacklisted_template".to_string(), "1.0.0".to_string());
    blk_meta.description = "Blacklisted template description".to_string();
    blk_meta.category = Some("malware_cat".to_string());
    blk_meta.tags = vec!["banned_tag".to_string()];
    let blk_cbor = blk_meta.to_cbor().expect("blk cbor");
    let blk_hash = blk_meta.hash().expect("blk hash").to_hex();

    for (addr, name, hash) in [
        (&visible_addr, "visible_template", visible_hash),
        (&blacklisted_addr, "blacklisted_template", blk_hash),
    ] {
        db::templates::upsert_template(
            &app.pool,
            &NewTemplate {
                template_address: *addr,
                template_name: name.to_string(),
                author_public_key: hex::encode(author_pk.as_bytes()),
                binary_hash: "cc".repeat(32),
                at_epoch: 30,
                metadata_hash: Some(hash),
            },
        )
        .await
        .expect("insert template");
    }

    // Submit metadata for both before blacklisting
    for (addr, cbor) in [
        (&visible_addr, visible_cbor.clone()),
        (&blacklisted_addr, blk_cbor.clone()),
    ] {
        let resp = app
            .client
            .post(format!(
                "{}/api/templates/{}/metadata",
                app.base_url,
                canonical_addr(addr)
            ))
            .body(cbor)
            .send()
            .await
            .expect("submit metadata");
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Mark both as featured, then blacklist blacklisted_addr
    db::templates::set_featured(&app.pool, &visible_addr, true, Some(1))
        .await
        .expect("feature visible");
    db::templates::set_featured(&app.pool, &blacklisted_addr, true, Some(2))
        .await
        .expect("feature blacklisted");
    db::templates::set_blacklisted(&app.pool, &blacklisted_addr, true)
        .await
        .expect("blacklist template");

    let blk_hex = canonical_addr(&blacklisted_addr);
    let vis_hex = canonical_addr(&visible_addr);

    // 3a. Absent from GET /api/templates/{addr} (returns 404)
    let resp = app
        .client
        .get(format!("{}/api/templates/{blk_hex}", app.base_url))
        .send()
        .await
        .expect("get blacklisted template");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = app
        .client
        .get(format!("{}/api/templates/{vis_hex}", app.base_url))
        .send()
        .await
        .expect("get visible template");
    assert_eq!(resp.status(), StatusCode::OK);

    // 3b. Absent from GET /api/search (unfiltered, by query, by address, by tag, by category)
    for query_str in [
        "".to_string(),
        "?q=blacklisted".to_string(),
        format!("?q={blk_hex}"),
        "?tags=banned_tag".to_string(),
        "?category=malware_cat".to_string(),
    ] {
        let resp = app
            .client
            .get(format!("{}/api/search{query_str}", app.base_url))
            .send()
            .await
            .expect("search request");
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.expect("search json");
        let results = body["results"].as_array().expect("results array");
        assert!(
            results.iter().all(|r| r["template_address"] != blk_hex),
            "blacklisted template leaked in /api/search{query_str}"
        );
    }

    // 3c. Absent from GET /api/templates/featured
    let resp = app
        .client
        .get(format!("{}/api/templates/featured", app.base_url))
        .send()
        .await
        .expect("featured request");
    assert_eq!(resp.status(), StatusCode::OK);
    let featured: Vec<Value> = resp.json().await.expect("featured json");
    assert_eq!(featured.len(), 1);
    assert_eq!(featured[0]["template_address"], vis_hex);

    // 3d. Metadata submission and signed metadata submission reject blacklisted template with 404
    let resp = app
        .client
        .post(format!("{}/api/templates/{blk_hex}/metadata", app.base_url))
        .body(blk_cbor.clone())
        .send()
        .await
        .expect("post metadata to blacklisted");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let (nonce_bytes, sig_bytes) =
        sign_metadata_payload(&author_sk, &author_pk, 77, &blacklisted_addr, &blk_cbor);
    let resp = app
        .client
        .post(format!(
            "{}/api/templates/{blk_hex}/metadata/signed",
            app.base_url
        ))
        .json(&json!({
            "metadata_cbor": hex::encode(&blk_cbor),
            "public_nonce": hex::encode(&nonce_bytes),
            "signature": hex::encode(&sig_bytes),
        }))
        .send()
        .await
        .expect("post signed metadata to blacklisted");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 3e. Indexer definition fetch queue also excludes blacklisted templates
    let missing_defs = db::templates::get_without_definition(&app.pool, 50)
        .await
        .expect("get_without_definition");
    assert!(missing_defs.contains(&vis_hex));
    assert!(!missing_defs.contains(&blk_hex));
}

#[tokio::test]
async fn admin_routes_reject_missing_malformed_expired_and_wrong_secret_tokens() {
    let app = TestApp::spawn().await;
    let stats_url = format!("{}/api/admin/stats", app.base_url);

    // 4a. Missing Authorization header -> 401 Unauthorized
    let resp = app.client.get(&stats_url).send().await.expect("no auth");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 4b. Malformed Authorization headers -> 401 Unauthorized
    for header_val in ["Basic dXNlcjpwYXNz", "Bearer", "Bearer not.a.valid.jwt"] {
        let resp = app
            .client
            .get(&stats_url)
            .header("Authorization", header_val)
            .send()
            .await
            .expect("malformed auth");
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "header '{header_val}' should be rejected with 401"
        );
    }

    // 4c. Expired JWT token -> 401 Unauthorized
    let expired_claims = Claims {
        sub: "admin".to_string(),
        exp: (chrono::Utc::now() - chrono::Duration::hours(2)).timestamp() as usize,
    };
    let expired_token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(&app.jwt_secret),
    )
    .expect("encode expired jwt");
    let resp = app
        .client
        .get(&stats_url)
        .header("Authorization", format!("Bearer {expired_token}"))
        .send()
        .await
        .expect("expired auth");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 4d. Token signed with wrong secret -> 401 Unauthorized
    let valid_exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
    let wrong_secret_token = encode(
        &Header::default(),
        &Claims {
            sub: "admin".to_string(),
            exp: valid_exp,
        },
        &EncodingKey::from_secret(b"wrong-secret-key-that-does-not-match"),
    )
    .expect("encode wrong secret jwt");
    let resp = app
        .client
        .get(&stats_url)
        .header("Authorization", format!("Bearer {wrong_secret_token}"))
        .send()
        .await
        .expect("wrong secret auth");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 4e. Valid token -> 200 OK
    let valid_token = encode(
        &Header::default(),
        &Claims {
            sub: "admin".to_string(),
            exp: valid_exp,
        },
        &EncodingKey::from_secret(&app.jwt_secret),
    )
    .expect("encode valid jwt");
    let resp = app
        .client
        .get(&stats_url)
        .header("Authorization", format!("Bearer {valid_token}"))
        .send()
        .await
        .expect("valid auth");
    assert_eq!(resp.status(), StatusCode::OK);
}
