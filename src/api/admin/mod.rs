use crate::db::{CanteenOperations, MenuOperations};
use actix_web::{guard, web};
use canteen::*;
use menu::*;
use utoipa_actix_web::{scope, service_config::ServiceConfig};

mod canteen;
mod menu;

pub fn config(cfg: &mut ServiceConfig, menu_ops: &MenuOperations, canteen_ops: &CanteenOperations) {
    cfg.service(
        scope::scope("/menu")
            .guard(guard::Header("content-type", "application/json"))
            .app_data(web::Data::new(menu_ops.clone()))
            .service(get_all_menu_items)
            .service(get_menu_item)
            .service(create_menu_item)
            .service(remove_menu_item)
            .service(update_menu_item),
    )
    .service(
        scope::scope("/canteen")
            .guard(guard::Header("content-type", "application/json"))
            .app_data(web::Data::new(canteen_ops.clone()))
            .service(create_canteen)
            .service(get_all_canteens),
    );
}
