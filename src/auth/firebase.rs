use crate::auth::config::FirebaseAuthConfig;
use crate::auth::jwks::JwksCache;
use jsonwebtoken::{decode, decode_header, Algorithm, Validation};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum FirebaseAuthError {
    #[error("token header error: {0}")]
    Header(String),
    #[error("jwks error: {0}")]
    Jwks(String),
    #[error("verification error: {0}")]
    Verify(String),
    #[error("claim mismatch: {0}")]
    Claim(String),
}

#[derive(Deserialize, Debug, Default)]
struct FirebaseInfo {
    #[serde(default)]
    sign_in_provider: String,
}

#[derive(Deserialize, Debug, Default)]
struct FirebaseContainer {
    #[serde(default)]
    firebase: FirebaseInfo,
}

#[derive(Deserialize, Debug)]
pub struct FirebaseClaims {
    pub sub: String,
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: bool,
    pub name: Option<String>,
    #[serde(rename = "picture")]
    pub picture: Option<String>,
    #[serde(flatten)]
    fb: FirebaseContainer,
}

pub struct VerifiedFirebaseUser {
    pub uid: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub photo_url: Option<String>,
}

pub async fn verify_firebase_token(
    token: &str,
    cfg: &FirebaseAuthConfig,
    cache: &JwksCache,
) -> Result<VerifiedFirebaseUser, FirebaseAuthError> {
    let header = decode_header(token).map_err(|e| FirebaseAuthError::Header(e.to_string()))?;
    let kid = header
        .kid
        .ok_or_else(|| FirebaseAuthError::Header("kid missing".to_string()))?;
    if header.alg != Algorithm::RS256 {
        return Err(FirebaseAuthError::Claim("alg must be RS256".to_string()));
    }

    let key = cache
        .get_key(&kid)
        .await
        .map_err(|e| FirebaseAuthError::Jwks(e.to_string()))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[cfg.project_id.as_str()]);
    validation.set_issuer(&[&format!(
        "https://securetoken.google.com/{}",
        cfg.project_id
    )]);
    validation.leeway = cfg.leeway_secs;

    let data = decode::<FirebaseClaims>(token, &key, &validation)
        .map_err(|e| FirebaseAuthError::Verify(e.to_string()))?;

    let claims = data.claims;

    if cfg.require_google_provider && claims.fb.firebase.sign_in_provider != "google.com" {
        return Err(FirebaseAuthError::Claim("provider mismatch".to_string()));
    }

    if cfg.require_email_verified && !claims.email_verified {
        return Err(FirebaseAuthError::Claim("email not verified".to_string()));
    }

    if let (Some(domains), Some(email)) = (&cfg.allowed_domains, &claims.email) {
        let domain = email.split('@').nth(1).unwrap_or("").to_lowercase();
        if !domains.iter().any(|d| d == &domain) {
            return Err(FirebaseAuthError::Claim(
                "email domain not allowed".to_string(),
            ));
        }
    }

    Ok(VerifiedFirebaseUser {
        uid: claims.sub,
        email: claims.email,
        email_verified: claims.email_verified,
        display_name: claims.name,
        photo_url: claims.picture,
    })
}
