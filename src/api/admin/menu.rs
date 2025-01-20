use crate::db::MenuOperations;
use crate::enums::admin::{
    AllItemsResponse, ItemIdRequest, ItemResponse, NewItemResponse, ReduceStockRequest,
};
use crate::models::admin::{MenuItem, NewMenuItem};
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

#[put("/create")]
pub(super) async fn create_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<NewMenuItem>,
) -> impl Responder {
    let item_name = req_data.name.clone();
    match menu_ops.add_menu_item(req_data.into_inner()) {
        Ok(_) => {
            info!("New menu item created: {}", item_name);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(NewItemResponse {
            status: "error".to_string(),
            error: Some(e.to_string()),
        }),
    }
}

#[delete("/delete")]
pub(super) async fn remove_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<ItemIdRequest>,
) -> impl Responder {
    match menu_ops.remove_menu_item(req_data.into_inner().id) {
        Ok(x) => {
            info!("Menu item removed: {}", x.name);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(NewItemResponse {
            status: "error".to_string(),
            error: Some(e.to_string()),
        }),
    }
}

#[post("/enable")]
pub(super) async fn enable_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<ItemIdRequest>,
) -> impl Responder {
    match menu_ops.enable_menu_item(req_data.id) {
        Ok(x) => {
            info!("Menu item enabled: {}", x.name);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(NewItemResponse {
            status: "error".to_string(),
            error: Some(e.to_string()),
        }),
    }
}

#[post("/disable")]
pub(super) async fn disable_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<ItemIdRequest>,
) -> impl Responder {
    match menu_ops.disable_menu_item(req_data.id) {
        Ok(x) => {
            info!("Menu item disabled: {}", x.name);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(NewItemResponse {
            status: "error".to_string(),
            error: Some(e.to_string()),
        }),
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
        Err(e) => HttpResponse::InternalServerError().json(AllItemsResponse {
            status: "error".to_string(),
            data: Vec::new(),
            error: Some(e.to_string()),
        }),
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
        Err(e) => HttpResponse::InternalServerError().json(ItemResponse {
            status: "error".to_string(),
            data: MenuItem::default(),
            error: Some(e.to_string()),
        }),
    }
}

#[post("/buy")]
pub(super) async fn reduce_stock(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<ReduceStockRequest>,
) -> impl Responder {
    match menu_ops.reduce_stock(req_data.id, req_data.into_inner().amount as u32) {
        Ok(x) => {
            info!("Stock of {} reduced to {}", x.name, x.stock);
            HttpResponse::Ok().json(NewItemResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(NewItemResponse {
            status: "error".to_string(),
            error: Some(e.to_string()),
        }),
    }
}
