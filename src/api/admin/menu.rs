use crate::db::MenuOperations;
use crate::enums::admin::{AllItemsResponse, GeneralMenuResponse, ItemResponse, CreateMenuItemResponse, UpdateItemRequest};
use crate::models::admin::{MenuItem, NewMenuItem};
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};

#[utoipa::path(
    post,
    tag = "Menu",
    path = "/create",
    request_body = NewMenuItem,
    responses(
        (status = 200, description = "Menu item created", body = CreateMenuItemResponse),
        (status = 409, description = "Menu item could not be created", body = GeneralMenuResponse)
    ),
    summary = "Create a new menu item"
)]
#[post("/create")]
pub(super) async fn create_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<NewMenuItem>,
) -> impl Responder {
    let req_data = req_data.into_inner();
    let item_name = req_data.name.clone();
    match menu_ops.add_menu_item(req_data) {
        Ok(res) => {
            debug!("New menu item created: {} with id: {}", item_name, res.item_id);
            HttpResponse::Ok().json(CreateMenuItemResponse {
                status: "ok".to_string(),
                item_id: res.item_id,
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: create_menu_item(): {}", e.to_string());
            HttpResponse::Conflict().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    delete,
    tag = "Menu",
    path = "/delete/{id}",
    params(
        ("id", description = "Unique id of the item to delete"),
    ),
    responses(
        (status = 200, description = "Menu item deleted", body = GeneralMenuResponse),
        (status = 409, description = "Unable to delete menu item", body = GeneralMenuResponse)
    ),
    summary = "Delete an item from menu"
)]
#[delete("/delete/{id}")]
pub(super) async fn remove_menu_item(
    menu_ops: web::Data<MenuOperations>,
    path: web::Path<(i32,)>,
) -> impl Responder {
    let req_data = path.into_inner().0;
    match menu_ops.remove_menu_item(req_data) {
        Ok(x) => {
            debug!("Menu item removed: {}", x.name);
            HttpResponse::Ok().json(GeneralMenuResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: remove_menu_item(): {}", e.to_string());
            HttpResponse::Conflict().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    put,
    tag = "Menu",
    path = "/update",
    request_body = UpdateItemRequest,
    responses(
        (status = 200, description = "Menu item updated", body = GeneralMenuResponse),
        (status = 409, description = "Menu item cannot be updated", body = GeneralMenuResponse)
    ),
    summary = "Update an item in menu"
)]
#[put("/update")]
pub(super) async fn update_menu_item(
    menu_ops: web::Data<MenuOperations>,
    req_data: web::Json<UpdateItemRequest>,
) -> impl Responder {
    let req_data = req_data.into_inner();
    let update_data = req_data.update.clone();
    match menu_ops.update_menu_item(req_data.item_id, update_data.clone()) {
        Ok(x) => {
            debug!("Menu item updated: {}.\nChanges: {:?}", x.name, update_data);
            HttpResponse::Ok().json(GeneralMenuResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: update_menu_item(): {}", e.to_string());
            HttpResponse::Conflict().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    get,
    tag = "Menu",
    path = "/items",
    responses(
        (status = 200, description = "All menu items fetched", body = AllItemsResponse),
        (status = 500, description = "Menu items couldn't be fetched", body = AllItemsResponse)
    ),
    summary = "Fetch all menu items"
)]
#[get("/items")]
pub(super) async fn get_all_menu_items(menu_ops: web::Data<MenuOperations>) -> impl Responder {
    match menu_ops.get_all_menu_items() {
        Ok(x) => {
            debug!("Menu items fetched!");
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
        }
    }
}

#[utoipa::path(
    get,
    tag = "Menu",
    path = "/items/{id}",
    params(
        ("id", description = "Unique id of the item to fetch"),
    ),
    responses(
        (status = 200, description = "Specified menu item fetched", body = GeneralMenuResponse),
        (status = 409, description = "Specified menu item does not exist", body = GeneralMenuResponse)
    ),
    summary = "Fetch specified item from menu"
)]
#[get("/items/{id}")]
pub(super) async fn get_menu_item(
    menu_ops: web::Data<MenuOperations>,
    path: web::Path<(i32,)>,
) -> impl Responder {
    match menu_ops.get_menu_item(path.into_inner().0) {
        Ok(x) => {
            debug!("Menu item fetched: {}", x.name);
            HttpResponse::Ok().json(ItemResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!("MENU: get_menu_item(): {}", e.to_string());
            HttpResponse::BadRequest().json(ItemResponse {
                status: "error".to_string(),
                data: MenuItem::default(),
                error: Some(e.to_string()),
            })
        }
    }
}
