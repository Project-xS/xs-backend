use crate::auth::config::AdminJwtConfig;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum AdminJwtError {
    #[error("verification error: {0}")]
    Verify(String),
}

#[derive(Serialize, Deserialize)]
struct AdminClaims {
    iss: String,
    aud: String,
    sub: String, // canteen_id
    iat: u64,
    exp: u64,
}

pub fn issue_admin_jwt(canteen_id: i32, cfg: &AdminJwtConfig) -> Result<String, AdminJwtError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = AdminClaims {
        iss: cfg.issuer.clone(),
        aud: cfg.audience.clone(),
        sub: canteen_id.to_string(),
        iat: now,
        exp: now + cfg.expiry_secs,
    };
    let header = Header::new(Algorithm::HS256);
    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(cfg.secret.as_bytes()),
    )
    .map_err(|e| AdminJwtError::Verify(e.to_string()))
}

pub fn verify_admin_jwt(token: &str, cfg: &AdminJwtConfig) -> Result<i32, AdminJwtError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[cfg.issuer.as_str()]);
    validation.set_audience(&[cfg.audience.as_str()]);
    let data = decode::<AdminClaims>(
        token,
        &DecodingKey::from_secret(cfg.secret.as_bytes()),
        &validation,
    )
    .map_err(|e| AdminJwtError::Verify(e.to_string()))?;
    let id: i32 = data
        .claims
        .sub
        .parse()
        .map_err(|e| AdminJwtError::Verify(format!("invalid sub: {e}")))?;
    Ok(id)
}
