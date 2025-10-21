#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod api;
mod auth;
mod db;
mod enums;
mod models;
mod traits;

use crate::api::default_error_handler;
use crate::db::{
    establish_connection_pool, run_db_migrations, AssetOperations, CanteenOperations,
    MenuOperations, OrderOperations, SearchOperations, UserOperations,
};
use actix_web::{middleware, web, App, HttpServer};
use auth::{AdminJwtConfig, AuthLayer, FirebaseAuthConfig, JwksCache};
use dotenvy::dotenv;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::{BasicAuth, Config, SwaggerUi};

#[derive(Clone)]
pub(crate) struct AppState {
    user_ops: UserOperations,
    menu_ops: MenuOperations,
    canteen_ops: CanteenOperations,
    order_ops: OrderOperations,
    search_ops: SearchOperations,
    asset_ops: AssetOperations,
}

impl AppState {
    pub(crate) async fn new(url: &str) -> Self {
        let db = establish_connection_pool(url);
        run_db_migrations(db.clone()).expect("Unable to run migrations");
        let user_ops = UserOperations::new(db.clone()).await;
        let menu_ops = MenuOperations::new(db.clone()).await;
        let canteen_ops = CanteenOperations::new(db.clone()).await;
        let order_ops = OrderOperations::new(db.clone()).await;
        let search_ops = SearchOperations::new(db.clone()).await;
        let asset_ops = AssetOperations::new()
            .await
            .expect("Unable to create asset_upload operations");
        AppState {
            user_ops,
            menu_ops,
            canteen_ops,
            order_ops,
            search_ops,
            asset_ops,
        }
    }
}

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

    // Server configuration
    const HOST: &str = if cfg!(debug_assertions) {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    const PORT: u16 = 8080;

    info!("Starting server at {}:{}", HOST, PORT);

    HttpServer::new(move || {
        App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .configure(|cfg| {
                api::configure(cfg, &state);
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
