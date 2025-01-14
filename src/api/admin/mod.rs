use actix_web::{web, HttpResponse};

pub mod menu;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/menu")
            .app_data(web::JsonConfig::default().error_handler(|err, _| {
                info!("Error in admin menu: {}", err);
                actix_web::error::InternalError::from_response(
                    "",
                    HttpResponse::BadRequest()
                        .finish(),
                )
                    .into()
            }))
            .route("/items", web::get().to(menu::get_all_menu_items))
            .route("/item", web::get().to(menu::get_menu_item))
            .route("/create", web::put().to(menu::create_menu_item))
            .route("/delete", web::delete().to(menu::remove_menu_item))
            .route("/enable", web::post().to(menu::enable_menu_item))
            .route("/disable", web::post().to(menu::disable_menu_item))
            .route("/buy", web::post().to(menu::reduce_stock))
    );
}
