use crate::db::CanteenOperations;
use crate::enums::admin::{AllCanteenResponse, NewCanteenResponse};
use crate::models::admin::NewCanteen;
use actix_web::{get, put, web, HttpResponse, Responder};

#[put("/create")]
pub(super) async fn create_canteen(
    canteen_ops: web::Data<CanteenOperations>,
    req_data: web::Json<NewCanteen>,
) -> impl Responder {
    let item_name = req_data.canteen_name.clone();
    match canteen_ops.create_canteen(req_data.into_inner()) {
        Ok(_) => {
            info!("New canteen created: {}", item_name);
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
        },
    }
}

#[get("")]
pub(super) async fn get_all_canteens(menu_ops: web::Data<CanteenOperations>) -> impl Responder {
    match menu_ops.get_all_canteens() {
        Ok(x) => {
            info!("Canteens fetched!");
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
        },
    }
}
