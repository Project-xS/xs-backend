use crate::db::CanteenOperations;
use crate::enums::admin::{
    AllCanteenResponse, AllItemsResponse, GeneralMenuResponse, LoginRequest, NewCanteenResponse,
};
use crate::models::admin::NewCanteen;
use actix_web::{get, post, web, HttpResponse, Responder};
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
    req_data: web::Json<NewCanteen>,
) -> impl Responder {
    let item_name = req_data.canteen_name.clone();
    match canteen_ops.create_canteen(req_data.into_inner()) {
        Ok(_) => {
            debug!(
                "create_canteen: successfully created new canteen '{}'",
                item_name
            );
            HttpResponse::Ok().json(NewCanteenResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!(
                "create_canteen: failed to create canteen '{}': {}",
                item_name, e
            );
            HttpResponse::BadRequest().json(NewCanteenResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
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
pub(super) async fn get_all_canteens(menu_ops: web::Data<CanteenOperations>) -> impl Responder {
    match menu_ops.get_all_canteens() {
        Ok(x) => {
            debug!(
                "get_all_canteens: successfully fetched {} canteens",
                x.len()
            );
            HttpResponse::Ok().json(AllCanteenResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!("get_all_canteens: failed to retrieve canteens: {}", e);
            HttpResponse::InternalServerError().json(AllCanteenResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            })
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
) -> impl Responder {
    let search_canteen_id = path.into_inner().0;
    match menu_ops.get_canteen_items(search_canteen_id) {
        Ok(x) => {
            debug!(
                "get_canteen_menu: successfully fetched {} menu items of canteen {}",
                x.len(),
                search_canteen_id
            );
            HttpResponse::Ok().json(AllItemsResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!(
                "get_canteen_menu: failed to retrieve canteen items of {}: {}",
                search_canteen_id, e
            );
            HttpResponse::InternalServerError().json(AllCanteenResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    tag = "Canteen",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Successfully logged in", body = GeneralMenuResponse),
        (status = 401, description = "Incorrect username or password", body = GeneralMenuResponse),
        (status = 500, description = "Failed to retrieve login details due to server error", body = GeneralMenuResponse),
    ),
    summary = "Initiate login request for a canteen"
)]
#[post("/login")]
pub(super) async fn login_canteen(
    menu_ops: web::Data<CanteenOperations>,
    req_data: web::Json<LoginRequest>,
) -> impl Responder {
    match menu_ops.login_canteen(&req_data.username, &req_data.password) {
        Ok(login_status) => {
            if login_status {
                debug!(
                    "login_canteen: successfully logged in canteen {}",
                    &req_data.username
                );
                HttpResponse::Ok().json(GeneralMenuResponse {
                    status: "ok".to_string(),
                    error: None,
                })
            } else {
                debug!(
                    "login_canteen: incorrect password for canteen {}",
                    &req_data.username
                );
                HttpResponse::Unauthorized().json(GeneralMenuResponse {
                    status: "invalid_password".to_string(),
                    error: None,
                })
            }
        }
        Err(e) => {
            error!(
                "login_canteen: failed to login {}: {}",
                &req_data.username, e
            );
            HttpResponse::InternalServerError().json(GeneralMenuResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}
