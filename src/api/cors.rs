use actix_cors::Cors;

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn parse_allowed_origins() -> Vec<String> {
    std::env::var("CORS_ALLOWED_ORIGINS")
        .ok()
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub fn cors_middleware() -> Cors {
    let allowed_origins = parse_allowed_origins();
    let allow_credentials = env_flag_enabled("CORS_ALLOW_CREDENTIALS");

    let mut cors = Cors::default()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);

    if allowed_origins.is_empty() {
        cors = if allow_credentials {
            // When credentials are enabled, we cannot send wildcard origin.
            cors.allowed_origin_fn(|_, _| true)
        } else {
            cors.allow_any_origin()
        };
    } else {
        for origin in allowed_origins {
            cors = cors.allowed_origin(&origin);
        }
    }

    if allow_credentials {
        cors = cors.supports_credentials();
    }

    cors
}
