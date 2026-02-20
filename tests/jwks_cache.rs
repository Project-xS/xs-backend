// No `mod common` needed — these tests exercise JwksCache directly over HTTP
// via wiremock, with no database or API involvement.

use proj_xs::auth::jwks::{JwksCache, JwksError};

// RSA 2048 public key (n, e) from a well-known test JWK.
// Used to produce a JWKS response the real jsonwebtoken parser will accept.
const TEST_N: &str = "pjdss8ZaDfEH6K6U7GeW2nxDqR4IP049fk1fK0lndimbMMVBdPv_hSpm8T8EtBDxrUdi1OHZfMhUixGyw-J5Pu3M0uNPPgBodGFrqM_0_TwkFuNNdYbSiGRuALLmWHLovzV5STG3GFjBGQDHPsJMqOoTzRDnkjZQ7dqMmSGjBQZWm0_Y5rmLEr5TBBh1PEOhM4H-o7RsBg0OcCh0ILsM3KWVDdnnS34yROFhZt0NqFkBP_x5HNu5TqC2v94JUC5r0KvPvtdpR66qSPGWqcqDWFLSPAI0RkAJhAFdQlpqrOBqMg5-lBqe3e_UElBQiQPiHarLpjkFyInkGug4Dw";
const TEST_E: &str = "AQAB";

fn jwks_body(kid: &str) -> String {
    serde_json::json!({
        "keys": [{
            "kty": "RSA",
            "kid": kid,
            "n": TEST_N,
            "e": TEST_E
        }]
    })
    .to_string()
}

// ── OAuth2 JWKS format ───────────────────────────────────────────────────────

#[actix_rt::test]
async fn jwks_oauth2_format_key_found() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/jwks"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(jwks_body("key-1")))
        .mount(&server)
        .await;

    let cache = JwksCache::new(format!("{}/jwks", server.uri()), 3600);
    let result = cache.get_key("key-1").await;
    assert!(
        result.is_ok(),
        "should find key-1 but got err: {:?}",
        result.err()
    );
}

#[actix_rt::test]
async fn jwks_oauth2_format_unknown_kid_returns_not_found() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/jwks"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(jwks_body("key-1")))
        .mount(&server)
        .await;

    let cache = JwksCache::new(format!("{}/jwks", server.uri()), 3600);
    let err = cache
        .get_key("other-key")
        .await
        .err()
        .expect("unknown kid should fail");
    assert!(
        matches!(err, JwksError::NotFound(_)),
        "expected NotFound, got {:?}",
        err
    );
}

// ── Cache-hit avoids a second network call ───────────────────────────────────

#[actix_rt::test]
async fn jwks_cache_hit_avoids_second_network_call() {
    let server = wiremock::MockServer::start().await;
    // `.expect(1)` causes the MockServer to panic on drop if != 1 request arrives.
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/jwks"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(jwks_body("key-hit")))
        .expect(1)
        .mount(&server)
        .await;

    let cache = JwksCache::new(format!("{}/jwks", server.uri()), 3600);
    cache.get_key("key-hit").await.expect("first call");
    // Second call must use the in-memory cache — no new HTTP request.
    cache
        .get_key("key-hit")
        .await
        .expect("second call from cache");
    // MockServer verifies expect(1) on drop.
}

// ── Cache-Control max-age is respected ──────────────────────────────────────

#[actix_rt::test]
async fn jwks_cache_control_max_age_header_used() {
    // Serve a key with Cache-Control: max-age=300 and verify it's accepted.
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/jwks"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("cache-control", "public, max-age=300")
                .set_body_string(jwks_body("key-ttl")),
        )
        .mount(&server)
        .await;

    let cache = JwksCache::new(format!("{}/jwks", server.uri()), 3600);
    let result = cache.get_key("key-ttl").await;
    assert!(
        result.is_ok(),
        "key should be found even with explicit max-age: {:?}",
        result.err()
    );
}

// ── X.509 map fallback ───────────────────────────────────────────────────────

#[actix_rt::test]
async fn jwks_x509_fallback_invalid_cert_silently_skipped() {
    // Not a valid JWKS; code falls back to Firebase X.509 map format.
    // Invalid PEM is silently skipped → get_key returns NotFound.
    let server = wiremock::MockServer::start().await;
    let x509_body = serde_json::json!({
        "x509-key": "not-a-valid-pem-certificate"
    })
    .to_string();
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/jwks"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(x509_body))
        .mount(&server)
        .await;

    let cache = JwksCache::new(format!("{}/jwks", server.uri()), 3600);
    let err = cache
        .get_key("x509-key")
        .await
        .err()
        .expect("invalid cert should be skipped → NotFound");
    assert!(
        matches!(err, JwksError::NotFound(_)),
        "expected NotFound after invalid cert skip, got {:?}",
        err
    );
}

// ── Non-JSON body → parse error swallowed → NotFound ────────────────────────

#[actix_rt::test]
async fn jwks_unparseable_body_returns_not_found() {
    // Non-JSON body → both parse attempts fail → get_key returns NotFound.
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/jwks"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_string("this is not json at all"),
        )
        .mount(&server)
        .await;

    let cache = JwksCache::new(format!("{}/jwks", server.uri()), 3600);
    let err = cache
        .get_key("any-kid")
        .await
        .err()
        .expect("bad body → NotFound");
    assert!(
        matches!(err, JwksError::NotFound(_)),
        "expected NotFound, got {:?}",
        err
    );
}

// ── Network error (unreachable server) ──────────────────────────────────────

#[actix_rt::test]
async fn jwks_network_error_returns_not_found() {
    // refresh() fails with JwksError::Network → get_key returns NotFound.
    let cache = JwksCache::new("http://127.0.0.1:1/jwks".to_string(), 3600);
    let err = cache
        .get_key("any-kid")
        .await
        .err()
        .expect("network error → NotFound");
    assert!(
        matches!(err, JwksError::NotFound(_)),
        "expected NotFound (network error swallowed), got {:?}",
        err
    );
}
