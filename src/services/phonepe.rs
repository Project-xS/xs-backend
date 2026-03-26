use chrono::{DateTime, TimeZone, Utc};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const TOKEN_EXPIRY_SAFETY_SECS: i64 = 30;

#[derive(Clone, Debug)]
pub enum PhonePeMode {
    Sandbox,
    Production,
}

#[derive(Clone, Debug)]
pub struct PhonePeConfig {
    pub enabled: bool,
    pub mode: PhonePeMode,
    pub client_id: String,
    pub client_secret: String,
    pub client_version: String,
    pub merchant_id: String,
    pub order_expire_after_secs: i64,
    pub http_timeout_secs: u64,
    pub auth_base_url: String,
    pub pg_base_url: String,
    webhook_auth_hash_hex: Option<String>,
}

#[derive(Clone, Debug)]
struct CachedOAuthToken {
    token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct PhonePeClient {
    cfg: PhonePeConfig,
    http: reqwest::Client,
    token_cache: Arc<RwLock<Option<CachedOAuthToken>>>,
}

#[derive(Debug, Clone)]
pub struct PhonePeCreateOrderResult {
    pub phonepe_order_id: String,
    pub sdk_token: String,
    pub merchant_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateOrderRequest<'a> {
    merchant_order_id: &'a str,
    amount: i32,
    expire_after: i64,
    payment_flow: PaymentFlow,
    enabled_payment_modes: Vec<EnabledPaymentMode>,
}

#[derive(Debug, Clone, Serialize)]
struct PaymentFlow {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Clone, Serialize)]
struct EnabledPaymentMode {
    #[serde(rename = "type")]
    kind: String,
}

impl PhonePeConfig {
    pub fn from_env() -> Result<Self, String> {
        let enabled = std::env::var("PHONEPE_ENABLED")
            .ok()
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let mode = match std::env::var("PHONEPE_MODE")
            .unwrap_or_else(|_| "sandbox".to_string())
            .to_lowercase()
            .as_str()
        {
            "sandbox" => PhonePeMode::Sandbox,
            "production" => PhonePeMode::Production,
            other => return Err(format!("Invalid PHONEPE_MODE value: {}", other)),
        };

        let default_auth_base = match mode {
            PhonePeMode::Sandbox => "https://api-preprod.phonepe.com/apis/pg-sandbox",
            PhonePeMode::Production => "https://api.phonepe.com/apis/identity-manager",
        };
        let default_pg_base = match mode {
            PhonePeMode::Sandbox => "https://api-preprod.phonepe.com/apis/pg-sandbox",
            PhonePeMode::Production => "https://api.phonepe.com/apis/pg",
        };

        let auth_base_url = std::env::var("PHONEPE_AUTH_BASE_URL")
            .unwrap_or_else(|_| default_auth_base.to_string());
        let pg_base_url =
            std::env::var("PHONEPE_PG_BASE_URL").unwrap_or_else(|_| default_pg_base.to_string());

        let order_expire_after_secs = std::env::var("PHONEPE_ORDER_EXPIRE_AFTER_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1200);
        let http_timeout_secs = std::env::var("PHONEPE_HTTP_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(15);

        if !enabled {
            return Ok(Self {
                enabled,
                mode,
                client_id: String::new(),
                client_secret: String::new(),
                client_version: "1".to_string(),
                merchant_id: String::new(),
                order_expire_after_secs,
                http_timeout_secs,
                auth_base_url,
                pg_base_url,
                webhook_auth_hash_hex: None,
            });
        }

        let client_id = std::env::var("PHONEPE_CLIENT_ID")
            .map_err(|_| "PHONEPE_CLIENT_ID must be set when PHONEPE_ENABLED=true".to_string())?;
        let client_secret = std::env::var("PHONEPE_CLIENT_SECRET").map_err(|_| {
            "PHONEPE_CLIENT_SECRET must be set when PHONEPE_ENABLED=true".to_string()
        })?;
        let client_version = std::env::var("PHONEPE_CLIENT_VERSION").unwrap_or_else(|_| "1".into());
        let merchant_id = std::env::var("PHONEPE_MERCHANT_ID")
            .map_err(|_| "PHONEPE_MERCHANT_ID must be set when PHONEPE_ENABLED=true".to_string())?;

        let webhook_username = std::env::var("PHONEPE_WEBHOOK_USERNAME").map_err(|_| {
            "PHONEPE_WEBHOOK_USERNAME must be set when PHONEPE_ENABLED=true".to_string()
        })?;
        let webhook_password = std::env::var("PHONEPE_WEBHOOK_PASSWORD").map_err(|_| {
            "PHONEPE_WEBHOOK_PASSWORD must be set when PHONEPE_ENABLED=true".to_string()
        })?;
        let webhook_auth_hash_hex = Some(hash_sha256_hex(&format!(
            "{}:{}",
            webhook_username, webhook_password
        )));

        Ok(Self {
            enabled,
            mode,
            client_id,
            client_secret,
            client_version,
            merchant_id,
            order_expire_after_secs,
            http_timeout_secs,
            auth_base_url,
            pg_base_url,
            webhook_auth_hash_hex,
        })
    }

    fn trim_base(base: &str) -> &str {
        base.trim_end_matches('/')
    }
}

impl PhonePeClient {
    pub fn from_env() -> Result<Self, String> {
        let cfg = PhonePeConfig::from_env()?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.http_timeout_secs))
            .build()
            .map_err(|e| format!("failed to build HTTP client: {}", e))?;

        Ok(Self {
            cfg,
            http,
            token_cache: Arc::new(RwLock::new(None)),
        })
    }

    pub fn config(&self) -> &PhonePeConfig {
        &self.cfg
    }

    pub fn ensure_enabled(&self) -> Result<(), String> {
        if self.cfg.enabled {
            Ok(())
        } else {
            Err("PhonePe integration is disabled on server.".to_string())
        }
    }

    pub fn verify_webhook_header(&self, authorization_header: Option<&str>) -> bool {
        if !self.cfg.enabled {
            return false;
        }

        match (
            authorization_header.map(|v| v.trim()),
            self.cfg.webhook_auth_hash_hex.as_deref(),
        ) {
            (Some(provided), Some(expected)) => matches_expected_webhook_hash(provided, expected),
            _ => false,
        }
    }

    pub async fn create_sdk_order(
        &self,
        merchant_order_id: &str,
        amount: i32,
        expire_after_secs: i64,
    ) -> Result<PhonePeCreateOrderResult, String> {
        self.ensure_enabled()?;

        let token = self.get_oauth_token().await?;
        let url = format!(
            "{}/checkout/v2/sdk/order",
            PhonePeConfig::trim_base(&self.cfg.pg_base_url)
        );

        let req = CreateOrderRequest {
            merchant_order_id,
            amount,
            expire_after: expire_after_secs,
            payment_flow: PaymentFlow {
                kind: "PG_CHECKOUT".to_string(),
            },
            enabled_payment_modes: vec![EnabledPaymentMode {
                kind: "UPI_INTENT".to_string(),
            }],
        };

        let body = serde_json::to_string(&req)
            .map_err(|e| format!("failed to serialize create order payload: {}", e))?;
        let resp = self
            .http
            .post(url)
            .header(AUTHORIZATION, format!("O-Bearer {}", token))
            .header(CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| format!("PhonePe create-order request failed: {}", e))?;

        let status = resp.status();
        let resp_text = resp
            .text()
            .await
            .map_err(|e| format!("failed to read create-order response: {}", e))?;
        if !status.is_success() {
            return Err(format!(
                "PhonePe create-order returned {}: {}",
                status, resp_text
            ));
        }

        let value: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("invalid create-order response JSON: {}", e))?;

        let phonepe_order_id = extract_string_from_paths(
            &value,
            &[
                &["orderId"],
                &["order_id"],
                &["data", "orderId"],
                &["data", "order_id"],
                &["payload", "orderId"],
                &["payload", "order_id"],
            ],
        )
        .ok_or_else(|| "PhonePe create-order response missing orderId".to_string())?;

        let sdk_token = extract_string_from_paths(
            &value,
            &[
                &["token"],
                &["data", "token"],
                &["payload", "token"],
                &["sdkToken"],
                &["data", "sdkToken"],
            ],
        )
        .ok_or_else(|| "PhonePe create-order response missing token".to_string())?;

        let merchant_id = extract_string_from_paths(
            &value,
            &[
                &["merchantId"],
                &["merchant_id"],
                &["data", "merchantId"],
                &["data", "merchant_id"],
            ],
        )
        .unwrap_or_else(|| self.cfg.merchant_id.clone());

        Ok(PhonePeCreateOrderResult {
            phonepe_order_id,
            sdk_token,
            merchant_id,
        })
    }

    pub async fn fetch_order_state(&self, merchant_order_id: &str) -> Result<String, String> {
        self.ensure_enabled()?;

        let token = self.get_oauth_token().await?;
        let url = format!(
            "{}/checkout/v2/order/{}/status",
            PhonePeConfig::trim_base(&self.cfg.pg_base_url),
            merchant_order_id
        );

        let resp = self
            .http
            .get(url)
            .header(AUTHORIZATION, format!("O-Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("PhonePe status request failed: {}", e))?;

        let status = resp.status();
        let resp_text = resp
            .text()
            .await
            .map_err(|e| format!("failed to read PhonePe status response: {}", e))?;
        if !status.is_success() {
            return Err(format!("PhonePe status returned {}: {}", status, resp_text));
        }

        let value: serde_json::Value = serde_json::from_str(&resp_text)
            .map_err(|e| format!("invalid PhonePe status response JSON: {}", e))?;

        extract_string_from_paths(
            &value,
            &[
                &["state"],
                &["data", "state"],
                &["payload", "state"],
                &["data", "payload", "state"],
            ],
        )
        .map(|v| v.to_uppercase())
        .ok_or_else(|| "PhonePe status response missing state".to_string())
    }

    async fn get_oauth_token(&self) -> Result<String, String> {
        {
            let cache = self.token_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                let threshold = Utc::now() + chrono::Duration::seconds(TOKEN_EXPIRY_SAFETY_SECS);
                if cached.expires_at > threshold {
                    return Ok(cached.token.clone());
                }
            }
        }

        let fetched = self.fetch_oauth_token().await?;
        let token = fetched.token.clone();
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(fetched);
        }
        Ok(token)
    }

    async fn fetch_oauth_token(&self) -> Result<CachedOAuthToken, String> {
        let url = format!(
            "{}/v1/oauth/token",
            PhonePeConfig::trim_base(&self.cfg.auth_base_url)
        );

        let form_body = format!(
            "client_id={}&client_secret={}&client_version={}&grant_type=client_credentials",
            urlencoding::encode(&self.cfg.client_id),
            urlencoding::encode(&self.cfg.client_secret),
            urlencoding::encode(&self.cfg.client_version),
        );

        let resp = self
            .http
            .post(url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form_body)
            .send()
            .await
            .map_err(|e| format!("PhonePe OAuth request failed: {}", e))?;

        let status = resp.status();
        let resp_text = resp
            .text()
            .await
            .map_err(|e| format!("failed to read OAuth response: {}", e))?;
        if !status.is_success() {
            return Err(format!("PhonePe OAuth returned {}: {}", status, resp_text));
        }

        let value: serde_json::Value =
            serde_json::from_str(&resp_text).map_err(|e| format!("invalid OAuth JSON: {}", e))?;

        let token = extract_string_from_paths(
            &value,
            &[
                &["access_token"],
                &["accessToken"],
                &["token"],
                &["data", "access_token"],
                &["data", "accessToken"],
                &["data", "token"],
            ],
        )
        .ok_or_else(|| "PhonePe OAuth response missing access token".to_string())?;

        let expires_at_epoch = extract_i64_from_paths(
            &value,
            &[&["expires_at"], &["data", "expires_at"], &["expiryAt"]],
        );
        let expires_in_secs = extract_i64_from_paths(
            &value,
            &[&["expires_in"], &["data", "expires_in"], &["expiresIn"]],
        );

        let expires_at = if let Some(raw_epoch) = expires_at_epoch {
            epoch_to_datetime(raw_epoch)?
        } else if let Some(expires_in) = expires_in_secs {
            Utc::now() + chrono::Duration::seconds(expires_in)
        } else {
            // Conservative fallback if API response omits expiry metadata.
            Utc::now() + chrono::Duration::minutes(5)
        };

        Ok(CachedOAuthToken { token, expires_at })
    }
}

/// TODO: Serde over whatever this mess is. we can use serde alias if we don't have the concrete path.
fn extract_string_from_paths(value: &serde_json::Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        let mut current = value;
        let mut found = true;
        for key in *path {
            if let Some(next) = current.get(*key) {
                current = next;
            } else {
                found = false;
                break;
            }
        }
        if found {
            if let Some(s) = current.as_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn extract_i64_from_paths(value: &serde_json::Value, paths: &[&[&str]]) -> Option<i64> {
    for path in paths {
        let mut current = value;
        let mut found = true;
        for key in *path {
            if let Some(next) = current.get(*key) {
                current = next;
            } else {
                found = false;
                break;
            }
        }
        if found {
            if let Some(num) = current.as_i64() {
                return Some(num);
            }
            if let Some(text) = current.as_str() {
                if let Ok(parsed) = text.parse::<i64>() {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

fn epoch_to_datetime(raw_epoch: i64) -> Result<DateTime<Utc>, String> {
    // PhonePe can return seconds or milliseconds.
    let (secs, nanos) = if raw_epoch > 9_999_999_999 {
        let secs = raw_epoch / 1_000;
        let rem_ms = raw_epoch % 1_000;
        (secs, (rem_ms as u32) * 1_000_000)
    } else {
        (raw_epoch, 0)
    };

    Utc.timestamp_opt(secs, nanos)
        .single()
        .ok_or_else(|| "invalid epoch timestamp from PhonePe response".to_string())
}

fn hash_sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn matches_expected_webhook_hash(provided: &str, expected_hex: &str) -> bool {
    let value = provided.trim();
    if value.eq_ignore_ascii_case(expected_hex) {
        return true;
    }

    let lower = value.to_ascii_lowercase();

    if lower.starts_with("sha256(") && value.ends_with(')') && value.len() > 8 {
        let inner = &value[7..value.len() - 1];
        if inner.eq_ignore_ascii_case(expected_hex) {
            return true;
        }
        return hash_sha256_hex(inner).eq_ignore_ascii_case(expected_hex);
    }

    for prefix in ["sha256 ", "sha256=", "sha256:"] {
        if lower.starts_with(prefix) && value.len() > prefix.len() {
            let tail = value[prefix.len()..].trim();
            if tail.eq_ignore_ascii_case(expected_hex) {
                return true;
            }
            if tail.starts_with('(') && tail.ends_with(')') && tail.len() > 2 {
                let inner = &tail[1..tail.len() - 1];
                if inner.eq_ignore_ascii_case(expected_hex) {
                    return true;
                }
                return hash_sha256_hex(inner).eq_ignore_ascii_case(expected_hex);
            }
        }
    }

    false
}
