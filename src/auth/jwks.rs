use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum JwksError {
    #[error("network error: {0}")]
    Network(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("key not found for kid: {0}")]
    NotFound(String),
}

#[derive(Clone, Deserialize)]
pub struct Jwk {
    pub kid: String,
    pub kty: String,
    pub n: String,
    pub e: String,
    pub alg: Option<String>,
    #[serde(rename = "use")]
    pub use_: Option<String>,
}

#[derive(Deserialize)]
struct JwksResp {
    keys: Vec<Jwk>,
}

#[derive(Clone)]
pub struct JwksCache {
    url: String,
    client: reqwest::Client,
    pub(crate) keys: Arc<RwLock<HashMap<String, jsonwebtoken::DecodingKey>>>,
    expiry: Arc<RwLock<Instant>>, // expiry for the whole JWKS set
    default_ttl: Duration,
}

impl JwksCache {
    pub fn new(url: String, default_ttl_secs: u64) -> Self {
        Self {
            url,
            client: reqwest::Client::builder()
                .use_rustls_tls()
                .build()
                .expect("reqwest client"),
            keys: Arc::new(RwLock::new(HashMap::new())),
            expiry: Arc::new(RwLock::new(Instant::now())),
            default_ttl: Duration::from_secs(default_ttl_secs),
        }
    }

    async fn refresh(&self) -> Result<(), JwksError> {
        let resp = self
            .client
            .get(&self.url)
            .send()
            .await
            .map_err(|e| JwksError::Network(e.to_string()))?;

        // Determine max-age if present
        let ttl = resp
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_max_age)
            .map(Duration::from_secs)
            .unwrap_or(self.default_ttl);

        let body = resp.text().await.map_err(|e| JwksError::Network(e.to_string()))?;
        let jwks: JwksResp = serde_json::from_str(&body).map_err(|e| JwksError::Parse(e.to_string()))?;

        let mut map = HashMap::new();
        for k in jwks.keys {
            // Only RSA is expected
            if let Ok(key) = jsonwebtoken::DecodingKey::from_rsa_components(&k.n, &k.e) {
                map.insert(k.kid, key);
            }
        }

        let mut w = self.keys.write().await;
        *w = map;
        let mut exp = self.expiry.write().await;
        *exp = Instant::now() + ttl;
        Ok(())
    }

    pub async fn get_key(&self, kid: &str) -> Result<jsonwebtoken::DecodingKey, JwksError> {
        {
            let r = self.keys.read().await;
            if let Some(k) = r.get(kid) {
                return Ok(k.clone());
            }
        }

        // refresh if expired or kid missing
        let expired = Instant::now() >= *self.expiry.read().await;
        if expired {
            let _ = self.refresh().await; // best-effort
        } else {
            // still try once for unknown kid
            let _ = self.refresh().await;
        }

        let r = self.keys.read().await;
        r.get(kid)
            .cloned()
            .ok_or_else(|| JwksError::NotFound(kid.to_string()))
    }
}

fn parse_max_age(header: &str) -> Option<u64> {
    for part in header.split(',') {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("max-age=") {
            if let Ok(v) = rest.parse::<u64>() {
                return Some(v);
            }
        }
    }
    None
}

