use actix_web::web;

pub mod account;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(account::login))
            .route("/create_user", web::post().to(account::create_user))
    );
}
