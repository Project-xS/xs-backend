use actix_web::web;
use actix_web::web::Data;
use canteen::*;
use menu::*;
use crate::db::{CanteenOperations, MenuOperations};

mod canteen;
mod menu;

pub fn config(cfg: &mut web::ServiceConfig, menu_ops: &MenuOperations, canteen_ops: &CanteenOperations) {
    cfg.service(
        web::scope("/menu")
            .app_data(Data::new(menu_ops.clone()))
            .service(get_all_menu_items)
            .service(get_menu_item)
            .service(create_menu_item)
            .service(remove_menu_item)
            .service(update_menu_item)
    )
    .service(
        web::scope("/canteen")
            .app_data(Data::new(canteen_ops.clone()))
            .service(create_canteen)
            .service(get_all_canteens)
    );
}
