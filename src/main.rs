#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use actix_web::{middleware, web, App, HttpServer};
use dotenvy::dotenv;
use proj_xs::api::default_error_handler;
use proj_xs::auth::{AdminJwtConfig, AuthLayer, FirebaseAuthConfig, JwksCache};
use proj_xs::{api, AppState};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::{BasicAuth, Config, SwaggerUi};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        } else {
            openapi.components = Some(
                utoipa::openapi::ComponentsBuilder::new()
                    .security_scheme(
                        "bearer_auth",
                        SecurityScheme::Http(
                            HttpBuilder::new()
                                .scheme(HttpAuthScheme::Bearer)
                                .bearer_format("JWT")
                                .build(),
                        ),
                    )
                    .build(),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    security(("bearer_auth" = [])),
    tags((name = "Proj-xS", description = "endpoints."))
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenv() {
        error!("Failed to load .env file: {}. Defaulting to env vars...", e);
    }

    // Setup logging
    if std::env::var("RUST_LOG").is_err() {
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    pretty_env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // App State initialization & App Connection
    let state = AppState::new(database_url.as_str()).await;

    // Auth config
    let fb_cfg = FirebaseAuthConfig::from_env();
    let admin_cfg = AdminJwtConfig::from_env();
    let jwks_cache = JwksCache::new(fb_cfg.jwks_url.clone(), fb_cfg.cache_ttl_secs);

    // QR config
    let qr_secret =
        std::env::var("DELIVER_QR_HASH_SECRET").expect("DELIVER_QR_HASH_SECRET must be set");
    let qr_max_age: u64 = std::env::var("QR_TOKEN_MAX_AGE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(86400); // 24 hours default

    // Spawn background task to clean up expired holds
    {
        let hold_ops = state.hold_ops.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                match web::block({
                    let hold_ops = hold_ops.clone();
                    move || hold_ops.cleanup_expired_holds()
                })
                .await
                {
                    Ok(Ok(count)) => {
                        if count > 0 {
                            info!("Background cleanup: released {} expired order holds", count);
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Background cleanup error: {}", e);
                    }
                    Err(e) => {
                        error!("Background cleanup blocking error: {}", e);
                    }
                }
            }
        });
    }

    // Server configuration
    const HOST: &str = if cfg!(debug_assertions) {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    const PORT: u16 = 8080;

    info!("Starting server at {}:{}", HOST, PORT);

    HttpServer::new(move || {
        let qr_cfg = api::common::qr::QrConfig {
            secret: qr_secret.clone(),
            max_age_secs: qr_max_age,
        };

        App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .configure(|cfg| {
                api::configure(cfg, &state, qr_cfg);
            })
            .map(|app| {
                app.wrap(AuthLayer::new(
                    fb_cfg.clone(),
                    admin_cfg.clone(),
                    jwks_cache.clone(),
                    state.user_ops.clone(),
                ))
                .wrap(middleware::Logger::new("%r - %s - %Dms"))
            })
            .app_data(web::Data::new(fb_cfg.clone()))
            .app_data(web::Data::new(jwks_cache.clone()))
            .app_data(web::Data::new(state.user_ops.clone()))
            .app_data(web::Data::new(admin_cfg.clone()))
            .app_data(web::JsonConfig::default().error_handler(default_error_handler))
            .openapi_service(|api| {
                let base_cfg = Config::default().persist_authorization(true);
                let cfg = if let (Ok(u), Ok(p)) = (
                    std::env::var("SWAGGER_BASIC_USERNAME"),
                    std::env::var("SWAGGER_BASIC_PASSWORD"),
                ) {
                    base_cfg.basic_auth(BasicAuth {
                        username: u,
                        password: p,
                    })
                } else {
                    base_cfg
                };

                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", api)
                    .config(cfg)
            })
            .into_app()
    })
    .bind((HOST, PORT))?
    .run()
    .await
}
