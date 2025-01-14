use actix_web::{web, HttpResponse};

pub mod account;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                info!("Error in user auth: {}", err);
                actix_web::error::InternalError::from_response(
                    "",
                    HttpResponse::BadRequest()
                        .finish(),
                )
                .into()
            }))
            .route("/login", web::post().to(account::login))
            .route("/create_user", web::post().to(account::create_user))
    );
}
