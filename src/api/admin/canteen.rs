use crate::db::CanteenOperations;
use crate::enums::admin::{AllCanteenResponse, NewCanteenResponse};
use crate::models::admin::NewCanteen;
use actix_web::{get, post, web, HttpResponse, Responder};

#[utoipa::path(
    post,
    tag = "Canteen",
    path = "/create",
    request_body = NewCanteen,
    responses(
        (status = 200, description = "Canteen created", body = NewCanteenResponse)
    ),
    summary = "Create a new canteen"
)]
#[post("/create")]
pub(super) async fn create_canteen(
    canteen_ops: web::Data<CanteenOperations>,
    req_data: web::Json<NewCanteen>,
) -> impl Responder {
    let item_name = req_data.canteen_name.clone();
    match canteen_ops.create_canteen(req_data.into_inner()) {
        Ok(_) => {
            debug!("New canteen created: {}", item_name);
            HttpResponse::Ok().json(NewCanteenResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("CANTEEN: create_canteen(): {}", e.to_string());
            HttpResponse::InternalServerError().json(NewCanteenResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    get,
    tag = "Canteen",
    path = "",
    responses(
        (status = 200, description = "Fetched all available canteens", body = AllCanteenResponse)
    ),
    summary = "Fetch all available canteens"
)]
#[get("")]
pub(super) async fn get_all_canteens(menu_ops: web::Data<CanteenOperations>) -> impl Responder {
    match menu_ops.get_all_canteens() {
        Ok(x) => {
            debug!("Canteens fetched!");
            HttpResponse::Ok().json(AllCanteenResponse {
                status: "ok".to_string(),
                data: x,
                error: None,
            })
        }
        Err(e) => {
            error!("CANTEEN: get_all_canteens(): {}", e.to_string());
            HttpResponse::InternalServerError().json(AllCanteenResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string()),
            })
        }
    }
}
