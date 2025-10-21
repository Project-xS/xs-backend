use dotenvy::var;

#[derive(Clone)]
pub struct FirebaseAuthConfig {
    pub project_id: String,
    pub jwks_url: String,
    pub leeway_secs: u64,
    pub cache_ttl_secs: u64,
    pub require_google_provider: bool,
    pub require_email_verified: bool,
    pub allowed_domains: Option<Vec<String>>,
}

impl FirebaseAuthConfig {
    pub fn from_env() -> Self {
        let project_id =
            var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID must be set for Firebase auth");
        let jwks_url = var("FIREBASE_JWKS_URL")
            .unwrap_or_else(|_| "https://www.googleapis.com/oauth2/v3/certs".to_string());
        let leeway_secs = var("FIREBASE_LEEWAY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60);
        let cache_ttl_secs = var("FIREBASE_JWKS_CACHE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3600);
        let require_google_provider = var("FIREBASE_REQUIRE_GOOGLE_PROVIDER")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(true);
        let require_email_verified = var("FIREBASE_REQUIRE_EMAIL_VERIFIED")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(true);
        let allowed_domains = var("ALLOWED_GOOGLE_DOMAINS").ok().map(|v| {
            v.split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        });
        Self {
            project_id,
            jwks_url,
            leeway_secs,
            cache_ttl_secs,
            require_google_provider,
            require_email_verified,
            allowed_domains,
        }
    }
}

#[derive(Clone)]
pub struct AdminJwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub expiry_secs: u64,
}

impl AdminJwtConfig {
    pub fn from_env() -> Self {
        let secret = var("ADMIN_JWT_SECRET").expect("ADMIN_JWT_SECRET must be set");
        let issuer = var("ADMIN_JWT_ISSUER").unwrap_or_else(|_| "canteen-auth".to_string());
        let audience = var("ADMIN_JWT_AUDIENCE").unwrap_or_else(|_| "admin".to_string());
        // 12 hours expiry from user confirmation
        let expiry_secs = var("ADMIN_JWT_EXPIRY_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(12 * 60 * 60);
        Self {
            secret,
            issuer,
            audience,
            expiry_secs,
        }
    }
}
