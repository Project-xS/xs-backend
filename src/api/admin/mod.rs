use crate::api::ContentTypeHeader;
use crate::db::{AssetOperations, CanteenOperations, MenuOperations};
use crate::services::canteen_scheduler::CanteenSchedulerNotifier;
use actix_web::middleware::NormalizePath;
use actix_web::web;
use asset_management::*;
use canteen::*;
use menu::*;
use utoipa_actix_web::{scope, service_config::ServiceConfig};

mod asset_management;
mod canteen;
mod menu;

pub fn config(
    cfg: &mut ServiceConfig,
    menu_ops: &MenuOperations,
    canteen_ops: &CanteenOperations,
    asset_ops: &AssetOperations,
    scheduler: &CanteenSchedulerNotifier,
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
                    .service(remove_menu_item)
                    .service(upload_menu_item_pic)
                    .service(set_menu_pic_link),
            ),
    )
    .service(
        scope::scope("/canteen")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(canteen_ops.clone()))
            .app_data(web::Data::new(scheduler.clone()))
            .service(
                scope::scope("")
                    .guard(ContentTypeHeader)
                    .service(create_canteen)
                    .service(login_canteen),
            )
            .service(
                scope::scope("")
                    .service(upload_canteen_pic)
                    .service(set_canteen_pic_link)
                    .service(open_canteen)
                    .service(close_canteen)
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
