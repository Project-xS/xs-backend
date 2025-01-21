#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod api;
mod db;
mod enums;
mod models;

use crate::api::default_error_handler;
use crate::db::{establish_connection_pool, CanteenOperations, MenuOperations, UserOperations};
use actix_web::{middleware, web, App, HttpServer};
use dotenvy::dotenv;
use utoipa::OpenApi;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub(crate) struct AppState {
    pub user_ops: UserOperations,
    pub menu_ops: MenuOperations,
    pub canteen_ops: CanteenOperations,
}

impl AppState {
    pub(crate) fn new(url: &str) -> Self {
        let db = establish_connection_pool(url);
        let user_ops = UserOperations::new(db.clone());
        let menu_ops = MenuOperations::new(db.clone());
        let canteen_ops = CanteenOperations::new(db.clone());
        AppState {
            user_ops,
            menu_ops,
            canteen_ops,
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "Proj-xS", description = "endpoints.")
    )
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenv() {
        error!("Failed to load .env file: {}. Defaulting to env vars...", e);
    }

    // Setup logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Database Connection
    info!("Initializing database connection pool...");
    let state = AppState::new(database_url.as_str());

    // Server configuration
    const HOST: &str = if cfg!(debug_assertions) {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    const PORT: u16 = 8080;

    info!("Starting server at http://{}:{}", HOST, PORT);

    HttpServer::new(move || {
        App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .configure(|cfg| {
                api::configure(cfg, &state);
            })
            .map(|app| app.wrap(middleware::Logger::new("%r - %s - %Dms")))
            .app_data(web::JsonConfig::default().error_handler(default_error_handler))
            .openapi_service(|api| {
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", api)
            })
            .into_app()
    })
    .bind((HOST, PORT))?
    .run()
    .await
}
