use crate::auth::AdminPrincipal;
use crate::db::MenuOperations;
use crate::enums::admin::{
    AllItemsResponse, CreateMenuItemResponse, GeneralMenuResponse, ItemResponse, MenuItemWithPic,
    UpdateItemRequest, UploadMenuItemPicPresignedResponse,
};
use crate::models::admin::NewMenuItem;
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use log::{debug, error};

#[utoipa::path(
    tag = "Menu",
    request_body = NewMenuItem,
    responses(
        (status = 200, description = "Menu item successfully created", body = CreateMenuItemResponse),
        (status = 409, description = "Failed to create menu item due to conflict", body = GeneralMenuResponse)
    ),
    summary = "Add a new menu item to the menu"
)]
#[post("/create")]
pub(super) async fn create_menu_item(
    menu_ops: web::Data<MenuOperations>,
    _admin: AdminPrincipal,
    req_data: web::Json<NewMenuItem>,
) -> actix_web::Result<impl Responder> {
    let req_data = req_data.into_inner();
    let item_name = req_data.name.clone();
    let result = web::block(move || menu_ops.add_menu_item(req_data)).await?;
    match result {
        Ok(res) => {
            debug!(
                "create_menu_item: successfully created menu item with item_name '{}' and id {}",
                item_name, res.item_id
            );
            Ok(HttpResponse::Ok().json(CreateMenuItemResponse {
                status: "ok".to_string(),
                item_id: res.item_id,
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "create_menu_item: failed to create menu item '{}' due to error: {}",
                item_name, e
            );
            Ok(HttpResponse::Conflict().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Menu",
    params(
        ("id", description = "The unique identifier of the menu item to delete"),
    ),
    responses(
        (status = 200, description = "Menu item successfully deleted", body = GeneralMenuResponse),
        (status = 409, description = "Failed to delete menu item due to conflict", body = GeneralMenuResponse)
    ),
    summary = "Remove a menu item from the menu"
)]
#[delete("/delete/{id}")]
pub(super) async fn remove_menu_item(
    menu_ops: web::Data<MenuOperations>,
    _admin: AdminPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let req_data = path.into_inner().0;
    let result = web::block(move || menu_ops.remove_menu_item(req_data)).await?;
    match result {
        Ok(x) => {
            debug!(
                "remove_menu_item: successfully removed menu item '{}'",
                x.name
            );
            Ok(HttpResponse::Ok().json(GeneralMenuResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "remove_menu_item: failed to remove menu item with id {}: {}",
                req_data, e
            );
            Ok(HttpResponse::Conflict().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Menu",
    params(
        ("item_id", description = "The unique identifier of the item to upload pic for."),
    ),
    responses(
        (status = 200, description = "Presigned URL generated successfully", body = GeneralMenuResponse),
        (status = 409, description = "Failed to generate presigned url", body = GeneralMenuResponse)
    ),
    summary = "Get presigned URL for uploading the menu item picture. Call the resulting URL with PUT to upload the image."
)]
#[put("/upload_pic/{item_id}")]
pub(super) async fn upload_menu_item_pic(
    menu_ops: web::Data<MenuOperations>,
    _admin: AdminPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let item_id_to_set = path.into_inner().0;
    let result = menu_ops.upload_menu_item_pic(&item_id_to_set).await;
    match result {
        Ok(res) => {
            debug!(
                "upload_menu_item_pic: successfully generated presign upload for menu item '{}'",
                item_id_to_set
            );
            Ok(HttpResponse::Ok().json(UploadMenuItemPicPresignedResponse {
                status: "ok".to_string(),
                presigned_url: Some(res),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "upload_menu_item_pic: failed to generate presign upload for menu item with id {}: {}",
                item_id_to_set, e
            );
            Ok(
                HttpResponse::Conflict().json(UploadMenuItemPicPresignedResponse {
                    status: "error".to_string(),
                    presigned_url: None,
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

#[utoipa::path(
    tag = "Menu",
    params(
        ("item_id", description = "The unique identifier of the item to set pic for."),
    ),
    responses(
        (status = 200, description = "Menu item updated successfully", body = GeneralMenuResponse),
        (status = 409, description = "Failed to update menu item due to conflict", body = GeneralMenuResponse)
    ),
    summary = "Set picture link for a menu item after uploading the asset."
)]
#[put("/set_pic/{item_id}")]
pub(super) async fn set_menu_pic_link(
    menu_ops: web::Data<MenuOperations>,
    _admin: AdminPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let item_id_to_set = path.into_inner().0;
    let result = menu_ops.set_menu_item_pic(&item_id_to_set).await;
    match result {
        Ok(_x) => {
            debug!(
                "set_menu_pic_link: successfully approved pic for menu item '{}'",
                item_id_to_set
            );
            Ok(HttpResponse::Ok().json(GeneralMenuResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "set_menu_pic_link: failed to approve pic for menu item with id {}: {}",
                item_id_to_set, e
            );
            Ok(HttpResponse::Conflict().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Menu",
    request_body = UpdateItemRequest,
    responses(
        (status = 200, description = "Menu item updated successfully", body = GeneralMenuResponse),
        (status = 409, description = "Failed to update menu item due to conflict", body = GeneralMenuResponse)
    ),
    summary = "Modify an existing menu item"
)]
#[put("/update")]
pub(super) async fn update_menu_item(
    menu_ops: web::Data<MenuOperations>,
    _admin: AdminPrincipal,
    req_data: web::Json<UpdateItemRequest>,
) -> actix_web::Result<impl Responder> {
    let req_data = req_data.into_inner();
    let update_data = req_data.update;
    let update_data_cl = update_data.clone();
    let result =
        web::block(move || menu_ops.update_menu_item(req_data.item_id, update_data_cl)).await?;
    match result {
        Ok(x) => {
            debug!(
                "update_menu_item: successfully updated menu item '{}' with changes: {:?}",
                x.name, update_data
            );
            Ok(HttpResponse::Ok().json(GeneralMenuResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "update_menu_item: failed to update menu item with id {}: {}",
                req_data.item_id, e
            );
            Ok(HttpResponse::Conflict().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Menu",
    responses(
        (status = 200, description = "Successfully retrieved all menu items", body = AllItemsResponse),
        (status = 500, description = "Failed to retrieve menu items due to server error", body = AllItemsResponse)
    ),
    summary = "Retrieve the complete list of menu items"
)]
#[get("/items")]
pub(super) async fn get_all_menu_items(
    menu_ops: web::Data<MenuOperations>,
    _admin: AdminPrincipal,
) -> actix_web::Result<impl Responder> {
    let result = menu_ops.get_all_menu_items().await;
    match result {
        Ok(x) => {
            debug!(
                "get_all_menu_items: successfully fetched {} menu items",
                x.len()
            );
            Ok(HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            }))
        }
        Err(e) => {
            error!("get_all_menu_items: error retrieving menu items: {}", e);
            Ok(HttpResponse::InternalServerError().json(AllItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Menu",
    params(
        ("id", description = "Unique identifier of the menu item to retrieve"),
    ),
    responses(
        (status = 200, description = "Successfully fetched the specified menu item", body = GeneralMenuResponse),
        (status = 409, description = "Menu item not found", body = GeneralMenuResponse)
    ),
    summary = "Retrieve a specific menu item"
)]
#[get("/items/{id}")]
pub(super) async fn get_menu_item(
    menu_ops: web::Data<MenuOperations>,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let req_data = path.into_inner().0;
    let result = menu_ops.get_menu_item(req_data).await;
    match result {
        Ok(x) => {
            debug!("get_menu_item: successfully fetched menu item '{}'", x.name);
            Ok(HttpResponse::Ok().json(ItemResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "get_menu_item: failed to fetch menu item with id {}: {}",
                req_data, e
            );
            Ok(HttpResponse::BadRequest().json(ItemResponse {
                status: "error".to_string(),
                data: MenuItemWithPic::default(),
                error: Some(e.to_string()),
            }))
        }
    }
}
