use crate::db::OrderOperations;
use crate::enums::admin::ActiveItemCountResponse;
use crate::enums::users::{OrderRequest, OrderResponse};
use actix_web::{get, post, web, HttpResponse, Responder};

#[utoipa::path(
    post,
    tag = "Orders",
    path = "",
    request_body = OrderRequest,
    responses(
        (status = 200, description = "Order created successfully", body = OrderResponse)
    ),
    summary = "Create a new order"
)]
#[post("")]
pub(super) async fn create_order(
    order_ops: web::Data<OrderOperations>,
    req_data: web::Json<OrderRequest>,
) -> impl Responder {
    let OrderRequest { user_id, item_ids } = req_data.into_inner();
    match order_ops.create_order(user_id, item_ids.clone()) {
        Ok(_) => {
            debug!("Order created for user: {}, item: {:?}", user_id, item_ids);
            HttpResponse::Ok().json(OrderResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("ORDER: create_order(): {}", e.to_string());
            HttpResponse::BadRequest().json(OrderResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    get,
    tag = "Orders",
    path = "",
    responses(
        (status = 200, description = "Active common", body = ActiveItemCountResponse)
    ),
    summary = "Returns order -> count"
)]
#[get("")]
pub(super) async fn get_all_orders(order_ops: web::Data<OrderOperations>) -> impl Responder {
    let resp = order_ops.get_all_orders_by_count();
    HttpResponse::Ok().json(ActiveItemCountResponse {
        status: "ok".to_string(),
        data: resp,
        error: None
    })
}
