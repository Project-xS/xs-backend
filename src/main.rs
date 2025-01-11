#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod enums;
mod models;
mod db;
mod api;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use crate::db::UserOperations;
use dotenvy::dotenv;

#[get("/")]
async fn root_endpoint() -> impl Responder {
    HttpResponse::Ok().body("Server up!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if let Err(e) = dotenv() {
        eprintln!("Failed to load .env file: {}", e);
    }

    // Setup logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Database Connection
    info!("Initializing database connection pool...");
    let pool = db::establish_connection_pool(&database_url);

    let user_ops = UserOperations::new(pool.clone());

    // Server configuration
    const HOST: &str = "127.0.0.1";
    const PORT: u16 = 8080;

    info!("Starting server at http://{}:{}", HOST, PORT);

    HttpServer::new(move || {
        App::new()
            .service(root_endpoint)
            .configure(api::users::account::config)
            .app_data(web::Data::new(user_ops.clone()))
    })
        .bind((HOST, PORT))?
        .run()
        .await
}
