use crate::auth::AdminJwtConfig;
use crate::auth::admin_jwt::issue_admin_jwt;
use crate::db::CanteenOperations;
use crate::enums::admin::{
    AllCanteenResponse, AllItemsResponse, GeneralMenuResponse, LoginRequest, LoginResponse,
    NewCanteenResponse, UploadCanteenPicPresignedResponse,
};
use crate::models::admin::NewCanteen;
use actix_web::{get, post, put, web, HttpResponse, Responder};
use log::{debug, error};

#[utoipa::path(
    tag = "Canteen",
    request_body = NewCanteen,
    responses(
        (status = 200, description = "Canteen successfully created", body = NewCanteenResponse),
        (status = 400, description = "Failed to create canteen: invalid request or data error", body = NewCanteenResponse)
    ),
    summary = "Add a new canteen"
)]
#[post("/create")]
pub(super) async fn create_canteen(
    canteen_ops: web::Data<CanteenOperations>,
    _admin: crate::auth::AdminPrincipal,
    req_data: web::Json<NewCanteen>,
) -> actix_web::Result<impl Responder> {
    let item_name = req_data.canteen_name.clone();
    let result = web::block(move || canteen_ops.create_canteen(req_data.into_inner())).await?;
    match result {
        Ok(_) => {
            debug!(
                "create_canteen: successfully created new canteen '{}'",
                item_name
            );
            Ok(HttpResponse::Ok().json(NewCanteenResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "create_canteen: failed to create canteen '{}': {}",
                item_name, e
            );
            Ok(HttpResponse::BadRequest().json(NewCanteenResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Canteen",
    params(
        ("canteen_id", description = "The unique identifier of the canteen to upload pic for."),
    ),
    responses(
        (status = 200, description = "Presigned URL generated successfully", body = NewCanteenResponse),
        (status = 409, description = "Failed to generate presigned url", body = NewCanteenResponse)
    ),
    summary = "Get presigned URL for uploading the canteen picture. Call the resulting URL with PUT to upload the image."
)]
#[put("/upload_pic/{canteen_id}")]
pub(super) async fn upload_canteen_pic(
    canteen_ops: web::Data<CanteenOperations>,
    _admin: crate::auth::AdminPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let canteen_id_to_set = path.into_inner().0;
    let result = canteen_ops.upload_canteen_pic(&canteen_id_to_set).await;
    match result {
        Ok(res) => {
            debug!(
                "upload_canteen_pic_link:successfully generated presign upload for canteen '{}'",
                canteen_id_to_set
            );
            Ok(HttpResponse::Ok().json(UploadCanteenPicPresignedResponse {
                status: "ok".to_string(),
                presigned_url: Some(res),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "upload_canteen_pic_link: failed to generate presign upload for canteen with id {}: {}",
                canteen_id_to_set, e
            );
            Ok(
                HttpResponse::Conflict().json(UploadCanteenPicPresignedResponse {
                    status: "error".to_string(),
                    presigned_url: None,
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

#[utoipa::path(
    tag = "Canteen",
    params(
        ("canteen_id", description = "The unique identifier of the canteen to set pic for."),
    ),
    responses(
        (status = 200, description = "Canteen pic set successfully", body = NewCanteenResponse),
        (status = 409, description = "Failed to set pic for canteen due to conflict", body = NewCanteenResponse)
    ),
    summary = "Set picture link for a menu item after uploading the asset."
)]
#[put("/set_pic/{canteen_id}")]
pub(super) async fn set_canteen_pic_link(
    canteen_ops: web::Data<CanteenOperations>,
    _admin: crate::auth::AdminPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let canteen_id_to_set = path.into_inner().0;
    let result = canteen_ops.set_canteen_pic(&canteen_id_to_set).await;
    match result {
        Ok(_res) => {
            debug!(
                "set_canteen_pic_link: successfully approved pic for menu item '{}'",
                canteen_id_to_set
            );
            Ok(HttpResponse::Ok().json(NewCanteenResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "set_canteen_pic_link: failed to approve pic for menu item with id {}: {}",
                canteen_id_to_set, e
            );
            Ok(HttpResponse::Conflict().json(NewCanteenResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Canteen",
    responses(
        (status = 200, description = "Successfully retrieved all canteens", body = AllCanteenResponse),
        (status = 500, description = "Failed to retrieve canteens due to server error", body = AllCanteenResponse)
    ),
    summary = "Retrieve a list of all available canteens"
)]
#[get("")]
pub(super) async fn get_all_canteens(
    canteen_ops: web::Data<CanteenOperations>,
) -> actix_web::Result<impl Responder> {
    let result = canteen_ops.get_all_canteens().await;
    match result {
        Ok(x) => {
            debug!(
                "get_all_canteens: successfully fetched {} canteens",
                x.len()
            );
            Ok(HttpResponse::Ok().json(AllCanteenResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            }))
        }
        Err(e) => {
            error!("get_all_canteens: failed to retrieve canteens: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(AllCanteenResponse {
                    status: "error".to_string(),
                    data: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

#[utoipa::path(
    tag = "Canteen",
    responses(
        (status = 200, description = "Successfully retrieved the menu of canteen", body = AllItemsResponse),
        (status = 500, description = "Failed to retrieve menu of canteen due to server error", body = AllItemsResponse)
    ),
    summary = "Retrieve the menu of a canteen"
)]
#[get("/{id}/items")]
pub(super) async fn get_canteen_menu(
    menu_ops: web::Data<CanteenOperations>,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let search_canteen_id = path.into_inner().0;
    let result = menu_ops.get_canteen_items(search_canteen_id).await;
    match result {
        Ok(x) => {
            debug!(
                "get_canteen_menu: successfully fetched {} menu items of canteen {}",
                x.len(),
                search_canteen_id
            );
            Ok(HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "get_canteen_menu: failed to retrieve canteen items of {}: {}",
                search_canteen_id, e
            );
            Ok(
                HttpResponse::InternalServerError().json(AllCanteenResponse {
                    status: "error".to_string(),
                    data: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

#[utoipa::path(
    tag = "Canteen",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Successfully logged in", body = LoginResponse),
        (status = 401, description = "Incorrect username or password", body = LoginResponse),
        (status = 500, description = "Failed to retrieve login details due to server error", body = LoginResponse),
    ),
    summary = "Initiate login request for a canteen"
)]
#[post("/login")]
pub(super) async fn login_canteen(
    menu_ops: web::Data<CanteenOperations>,
    admin_cfg: web::Data<AdminJwtConfig>,
    req_data: web::Json<LoginRequest>,
) -> actix_web::Result<impl Responder> {
    let username_cl = req_data.username.clone();
    let password_cl = req_data.password.clone();
    let result = web::block(move || menu_ops.login_canteen(&username_cl, &password_cl)).await?;
    match result {
        Ok(login_status) => {
            if let Some(login_ok) = login_status {
                debug!(
                    "login_canteen: successfully logged in canteen {}",
                    &req_data.username
                );
                let token = issue_admin_jwt(login_ok.canteen_id, &admin_cfg)
                    .map_err(|_| actix_web::error::ErrorInternalServerError("jwt"))?;
                Ok(HttpResponse::Ok().json(LoginResponse {
                    status: "ok".to_string(),
                    data: Some(login_ok),
                    token: Some(token),
                    error: None,
                }))
            } else {
                debug!(
                    "login_canteen: incorrect password for canteen {}",
                    &req_data.username
                );
                Ok(HttpResponse::Unauthorized().json(LoginResponse {
                    status: "invalid_credentials".to_string(),
                    data: None,
                    token: None,
                    error: None,
                }))
            }
        }
        Err(e) => {
            error!(
                "login_canteen: failed to login {}: {}",
                &req_data.username, e
            );
            Ok(
                HttpResponse::InternalServerError().json(GeneralMenuResponse {
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}
