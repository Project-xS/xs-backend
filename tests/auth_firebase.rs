mod common;

use base64::Engine;
use proj_xs::auth::firebase::{verify_firebase_token, FirebaseAuthError};
use proj_xs::auth::{FirebaseAuthConfig, JwksCache};
use proj_xs::test_utils::init_test_env;

fn make_cfg() -> FirebaseAuthConfig {
    init_test_env();
    FirebaseAuthConfig {
        project_id: "test-project".to_string(),
        jwks_url: "http://localhost:1".to_string(), // unreachable — tests never reach key fetch
        leeway_secs: 60,
        cache_ttl_secs: 3600,
        require_google_provider: false,
        require_email_verified: false,
        allowed_domains: None,
    }
}

fn make_cache() -> JwksCache {
    JwksCache::new("http://localhost:1".to_string(), 3600)
}

// ── Header-level errors (caught before any JWKS lookup) ─────────────────────

#[actix_rt::test]
async fn firebase_rejects_completely_malformed_token() {
    let cfg = make_cfg();
    let cache = make_cache();
    let err = verify_firebase_token("not.a.jwt", &cfg, &cache)
        .await
        .err()
        .expect("should fail");
    assert!(
        matches!(err, FirebaseAuthError::Header(_)),
        "expected Header error, got {:?}",
        err
    );
}

#[actix_rt::test]
async fn firebase_rejects_token_without_kid() {
    // Craft a valid-looking JWT header with no "kid" field.
    // Header: {"alg":"RS256"} — no kid
    let header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"sub":"x"}"#);
    let fake_token = format!("{}.{}.fakesig", header, payload);

    let cfg = make_cfg();
    let cache = make_cache();
    let err = verify_firebase_token(&fake_token, &cfg, &cache)
        .await
        .err()
        .expect("should fail");
    assert!(
        matches!(err, FirebaseAuthError::Header(_)),
        "expected Header error for missing kid, got {:?}",
        err
    );
}

#[actix_rt::test]
async fn firebase_rejects_wrong_algorithm() {
    // Craft a JWT header with alg=HS256 (not RS256).
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"alg":"HS256","typ":"JWT","kid":"test-kid"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"sub":"x"}"#);
    let fake_token = format!("{}.{}.fakesig", header, payload);

    let cfg = make_cfg();
    let cache = make_cache();
    let err = verify_firebase_token(&fake_token, &cfg, &cache)
        .await
        .err()
        .expect("should fail");
    // Wrong alg is caught before JWKS lookup — reported as a Claim error.
    assert!(
        matches!(err, FirebaseAuthError::Claim(_)),
        "expected Claim error for wrong alg, got {:?}",
        err
    );
}

#[actix_rt::test]
async fn firebase_rejects_unknown_kid_via_jwks_error() {
    // Valid RS256 header with a kid, but the JwksCache has no keys and the
    // server at localhost:1 is unreachable, so get_key returns JwksError::NotFound.
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"alg":"RS256","typ":"JWT","kid":"unknown-kid"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"sub":"x"}"#);
    let fake_token = format!("{}.{}.fakesig", header, payload);

    let cfg = make_cfg();
    let cache = make_cache();
    let err = verify_firebase_token(&fake_token, &cfg, &cache)
        .await
        .err()
        .expect("should fail");
    assert!(
        matches!(err, FirebaseAuthError::Jwks(_)),
        "expected Jwks error for unknown kid, got {:?}",
        err
    );
}
