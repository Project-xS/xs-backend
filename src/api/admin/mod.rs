use crate::api::ContentTypeHeader;
use crate::db::{AssetUploadOperations, CanteenOperations, MenuOperations};
use actix_web::middleware::NormalizePath;
use actix_web::web;
use asset_upload::*;
use canteen::*;
use menu::*;
use utoipa_actix_web::{scope, service_config::ServiceConfig};

mod asset_upload;
mod canteen;
mod menu;

pub fn config(
    cfg: &mut ServiceConfig,
    menu_ops: &MenuOperations,
    canteen_ops: &CanteenOperations,
    asset_ops: &AssetUploadOperations,
) {
    cfg.service(
        scope::scope("/menu")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(menu_ops.clone()))
            .service(
                scope::scope("")
                    .guard(ContentTypeHeader)
                    .service(create_menu_item)
                    .service(update_menu_item),
            )
            .service(
                scope::scope("")
                    .service(get_all_menu_items)
                    .service(get_menu_item)
                    .service(remove_menu_item),
            ),
    )
    .service(
        scope::scope("/canteen")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(canteen_ops.clone()))
            .service(
                scope::scope("")
                    .guard(ContentTypeHeader)
                    .service(create_canteen)
                    .service(login_canteen),
            )
            .service(
                scope::scope("")
                    .service(get_all_canteens)
                    .service(get_canteen_menu),
            ),
    )
    .service(
        scope::scope("/assets")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(asset_ops.clone()))
            .service(upload_image_handler)
            .service(get_image_handler),
    );
}
