mod common;

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use proj_xs::auth::admin_jwt::{issue_admin_jwt, verify_admin_jwt};
use proj_xs::auth::AdminJwtConfig;
use serde_json;

fn test_jwt_config() -> AdminJwtConfig {
    common::setup_pool(); // ensures init_test_env() is called
    AdminJwtConfig::from_env()
}

#[test]
fn admin_jwt_issue_and_verify_round_trip() {
    let cfg = test_jwt_config();
    let canteen_id = 42;

    let token = issue_admin_jwt(canteen_id, &cfg).expect("issue jwt");
    let got_id = verify_admin_jwt(&token, &cfg).expect("verify jwt");
    assert_eq!(got_id, canteen_id);
}

#[test]
fn admin_jwt_wrong_secret_fails() {
    let cfg = test_jwt_config();
    let token = issue_admin_jwt(1, &cfg).expect("issue jwt");

    let bad_cfg = AdminJwtConfig {
        secret: "wrong-secret".to_string(),
        issuer: cfg.issuer.clone(),
        audience: cfg.audience.clone(),
        expiry_secs: cfg.expiry_secs,
    };
    assert!(verify_admin_jwt(&token, &bad_cfg).is_err());
}

#[test]
fn admin_jwt_expired_token_fails() {
    let cfg = test_jwt_config();
    // Build a token with exp=1 (ancient past) using the same secret
    let claims = serde_json::json!({
        "iss": cfg.issuer,
        "aud": cfg.audience,
        "sub": "1",
        "iat": 1u64,
        "exp": 1u64,
    });
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(cfg.secret.as_bytes()),
    )
    .expect("encode");
    assert!(verify_admin_jwt(&token, &cfg).is_err());
}

#[test]
fn admin_jwt_wrong_issuer_fails() {
    let cfg = test_jwt_config();
    let token = issue_admin_jwt(1, &cfg).expect("issue jwt");

    let bad_cfg = AdminJwtConfig {
        secret: cfg.secret.clone(),
        issuer: "wrong-issuer".to_string(),
        audience: cfg.audience.clone(),
        expiry_secs: cfg.expiry_secs,
    };
    assert!(verify_admin_jwt(&token, &bad_cfg).is_err());
}
