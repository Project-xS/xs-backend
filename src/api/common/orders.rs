use log::{debug, error};
use crate::db::OrderOperations;
use crate::enums::common::{ActiveItemCountResponse, DeliverOrderRequest, OrderItemContainer, OrderItemsResponse, OrderRequest, OrderResponse, OrdersItemsResponse};
use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, Debug, IntoParams)]
struct UserOrderQuery {
    user_id: Option<i32>,
    rfid: Option<String>,
}

#[utoipa::path(
    tag = "Orders",
    request_body = OrderRequest,
    responses(
        (status = 200, description = "Order successfully created", body = OrderResponse),
        (status = 409, description = "Failed to create order due to conflict or invalid items", body = OrderResponse)
    ),
    summary = "Place a new order"
)]
#[post("")]
pub(super) async fn create_order(
    order_ops: web::Data<OrderOperations>,
    req_data: web::Json<OrderRequest>,
) -> impl Responder {
    let OrderRequest { user_id, item_ids } = req_data.into_inner();
    match order_ops.create_order(user_id, item_ids.clone()) {
        Ok(_) => {
            debug!("create_order: successfully created order for user {} with item_ids {:?}", user_id, item_ids);
            HttpResponse::Ok().json(OrderResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("create_order: failed to create order for user {} with item_ids {:?}: {}", user_id, item_ids, e);
            HttpResponse::Conflict().json(OrderResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    tag = "Orders",
    responses(
        (status = 200, description = "Successfully fetched aggregated counts of active ordered items", body = ActiveItemCountResponse)
    ),
    summary = "Get aggregated active order item counts"
)]
#[get("")]
pub(super) async fn get_all_orders(order_ops: web::Data<OrderOperations>) -> impl Responder {
    let resp = order_ops.get_all_orders_by_count();
    HttpResponse::Ok().json(ActiveItemCountResponse {
        status: "ok".to_string(),
        data: resp,
        error: None,
    })
}

#[utoipa::path(
    tag = "Orders",
    params(
        ("id", description = "Unique identifier for the order"),
    ),
    responses(
        (status = 200, description = "Successfully retrieved items for the specified order", body = OrderItemsResponse)
    ),
    summary = "Get items for a specific order"
)]
#[get("/{id}")]
pub(super) async fn get_order_by_orderid(
    order_ops: web::Data<OrderOperations>,
    path: web::Path<(i32,)>,
) -> impl Responder {
    let search_order_id = path.into_inner().0;
    match order_ops.get_orders_by_orderid(&search_order_id) {
        Ok(data) => {
            debug!("get_order_by_orderid: retrieved {} items for order_id {}", data.items.len(), search_order_id);
            HttpResponse::Ok().json(OrderItemsResponse {
                status: "ok".to_string(),
                data,
                error: None,
            })
        },
        Err(e) => HttpResponse::InternalServerError().json(OrderItemsResponse {
            status: "error".to_string(),
            data: OrderItemContainer {
                order_id: search_order_id,
                items: Vec::new(),
            },
            error: Option::from(e.to_string()),
        }),
    }
}

#[utoipa::path(
    tag = "Orders",
    params(
        UserOrderQuery,
    ),
    responses(
        (status = 200, description = "Successfully retrieved order items for the specified user or RFID", body = OrderItemsResponse),
        (status = 500, description = "Failed to retrieve order items due to server error", body = OrderItemsResponse)
    ),
    summary = "Get active order items for a specified user or RFID"
)]
#[get("/by_user")]
pub(super) async fn get_orders_by_user(
    order_ops: web::Data<OrderOperations>,
    params: web::Query<UserOrderQuery>,
) -> impl Responder {
    if params.user_id.is_some() && params.rfid.is_some() {
        return HttpResponse::BadRequest().json(OrdersItemsResponse {
            status: "error".to_string(),
            error: Option::from("Cannot provide both user_id and rfid parameters".to_string()),
            data: Vec::new(),
        });
    }
    if let Some(search_user_id) = &params.user_id {
        match order_ops.get_orders_by_userid(search_user_id) {
            Ok(data) => {
                debug!("get_orders_by_user: retrieved {} orders for user_id {}", data.len(), search_user_id);
                HttpResponse::Ok().json(OrdersItemsResponse {
                    status: "ok".to_string(),
                    data,
                    error: None,
                })
            },
            Err(e) => {
                error!("get_orders_by_user: error retrieving orders for user_id {}: {}", search_user_id, e);
                HttpResponse::InternalServerError().json(OrdersItemsResponse {
                    status: "error".to_string(),
                    data: Vec::new(),
                    error: Some(e.to_string()),
                })
            }
        }
    } else if let Some(search_rfid) = &params.rfid {
        match order_ops.get_orders_by_rfid(search_rfid) {
            Ok(data) => {
                debug!("get_orders_by_user: retrieved {} orders for rfid '{}'", data.len(), search_rfid);
                HttpResponse::Ok().json(OrdersItemsResponse {
                    status: "ok".to_string(),
                    data,
                    error: None,
                })
            },
            Err(e) => {
                error!("get_orders_by_user: error retrieving orders for rfid '{}': {}", search_rfid, e);
                HttpResponse::InternalServerError().json(OrdersItemsResponse {
                    status: "error".to_string(),
                    data: Vec::new(),
                    error: Some(e.to_string()),
                })
            },
        }
    } else {
        HttpResponse::BadRequest().json(OrdersItemsResponse {
            status: "error".to_string(),
            error: Option::from("Either user_id or rfid must be provided".to_string()),
            data: Vec::new(),
        })
    }
}

#[utoipa::path(
    tag = "Orders",
    request_body = OrderRequest,
    responses(
        (status = 200, description = "Order delivered created", body = OrderResponse),
        (status = 409, description = "Failed to deliver order due to conflict or invalid items", body = OrderResponse)
    ),
    summary = "Deliver an existing order"
)]
#[post("/deliver")]
pub(super) async fn deliver_order(
    order_ops: web::Data<OrderOperations>,
    req_data: web::Json<DeliverOrderRequest>,
) -> impl Responder {
    let DeliverOrderRequest { order_id } = req_data.into_inner();
    match order_ops.deliver_order(&order_id) {
        Ok(_) => {
            debug!("deliver_order: successfully delivered order with order_id {:?}", order_id);
            HttpResponse::Ok().json(OrderResponse {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("create_order: failed to deliver order with order_id {:?}: {}", order_id, e);
            HttpResponse::Conflict().json(OrderResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}
