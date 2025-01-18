#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod api;
mod db;
mod enums;
mod models;

use crate::db::{CanteenOperations, MenuOperations, UserOperations};
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use dotenvy::dotenv;

#[get("/")]
async fn root_endpoint() -> impl Responder {
    HttpResponse::Ok().body("Server up!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenv() {
        info!("Failed to load .env file: {}. Defaulting to env vars...", e);
    }

    // Setup logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Database Connection
    info!("Initializing database connection pool...");
    let pool = db::establish_connection_pool(&database_url);

    let user_ops = UserOperations::new(pool.clone());
    let menu_ops = MenuOperations::new(pool.clone());
    let canteen_ops = CanteenOperations::new(pool.clone());

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
            .service(root_endpoint)
            .configure(api::users::config)
            .configure(api::admin::config)
            .app_data(web::Data::new(user_ops.clone()))
            .app_data(web::Data::new(menu_ops.clone()))
            .app_data(web::Data::new(canteen_ops.clone()))
    })
    .bind((HOST, PORT))?
    .run()
    .await
}
