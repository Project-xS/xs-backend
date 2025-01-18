use crate::api::errors::default_error_handler;
use actix_web::web;

pub mod account;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .app_data(web::JsonConfig::default().error_handler(default_error_handler))
            .route("/login", web::post().to(account::login))
            .route("/create_user", web::post().to(account::create_user))
    );
}
