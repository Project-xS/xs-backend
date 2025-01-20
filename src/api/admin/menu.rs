use crate::db::MenuOperations;
use crate::enums::admin::{AllItemsResponse, ItemIdRequest, ItemResponse, NewItemResponse, UpdateItemRequest};
use crate::models::admin::{MenuItem, NewMenuItem};
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

#[put("/create")]
pub(super) async fn create_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<NewMenuItem>,
) -> impl Responder {
    let req_data = req_data.into_inner();
    let item_name = req_data.name.clone();
    match menu_ops.add_menu_item(req_data) {
        Ok(_) => {
            info!("New menu item created: {}", item_name);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: create_menu_item(): {}", e.to_string());
            HttpResponse::InternalServerError().json(NewItemResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[delete("/delete")]
pub(super) async fn remove_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<ItemIdRequest>,
) -> impl Responder {
    let req_data = req_data.into_inner();
    match menu_ops.remove_menu_item(req_data.id) {
        Ok(x) => {
            info!("Menu item removed: {}", x.name);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: remove_menu_item(): {}", e.to_string());
            HttpResponse::InternalServerError().json(NewItemResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[post("/update")]
pub(super) async fn update_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<UpdateItemRequest>,
) -> impl Responder {
    let req_data = req_data.into_inner();
    let update_data = req_data.update.clone();
    match menu_ops.update_menu_item(req_data.item_id, update_data.clone()) {
        Ok(x) => {
            info!("Menu item updated: {}.\nChanges: {:?}", x.name, update_data);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: update_menu_item(): {}", e.to_string());
            HttpResponse::InternalServerError().json(NewItemResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        },
    }
}

#[get("/items")]
pub(super) async fn get_all_menu_items(menu_ops: web::Data<MenuOperations>) -> impl Responder {
    match menu_ops.get_all_menu_items() {
        Ok(x) => {
            info!("Menu items fetched!");
            HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: get_all_menu_items(): {}", e.to_string());
            HttpResponse::InternalServerError().json(AllItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            })
        },
    }
}

#[get("/item")]
pub(super) async fn get_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<ItemIdRequest>,
) -> impl Responder {
    match menu_ops.get_menu_item(req_data.id) {
        Ok(x) => {
            info!("Menu item fetched: {}", x.name);
            HttpResponse::Ok().json(ItemResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: get_menu_item(): {}", e.to_string());
            HttpResponse::InternalServerError().json(ItemResponse {
                status: "error".to_string(),
                data: MenuItem::default(),
                error: Some(e.to_string()),
            })
        },
    }
}
