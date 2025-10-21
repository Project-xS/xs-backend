use crate::auth::{AdminPrincipal, UserPrincipal};
use crate::db::OrderOperations;
use crate::enums::common::{
    OrderItemContainer, OrderItemsResponse, OrderRequest, OrderResponse, OrdersItemsResponse,
    TimedActiveItemCount, TimedActiveItemCountResponse,
};
use actix_web::{get, post, put, web, HttpResponse, Responder};
use log::{debug, error};
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, Debug, IntoParams)]
struct UserOrderQuery {
    user_id: Option<i32>,
    rfid: Option<String>,
}

#[derive(Deserialize, Debug, IntoParams)]
struct OrderCanteenQuery {
    canteen_id: i32,
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
    user: UserPrincipal,
    req_data: web::Json<OrderRequest>,
) -> actix_web::Result<impl Responder> {
    let OrderRequest {
        user_id: _ignored,
        deliver_at,
        item_ids,
    } = req_data.into_inner();
    let deliver_at_cl = deliver_at.clone();
    let item_ids_cl = item_ids.clone();
    if deliver_at.is_some()
        && (deliver_at != Some(String::from("11:00am - 12:00pm"))
            && deliver_at != Some(String::from("12:00pm - 01:00pm")))
    {
        return Ok(HttpResponse::BadRequest().json(OrderResponse {
            status: "error".to_string(),
            error: Some(format!(
                "Invalid time band: {}",
                deliver_at.unwrap_or(String::new())
            )),
        }));
    }
    let uid = user.user_id();
    let result = web::block(move || order_ops.create_order(uid, item_ids_cl, deliver_at_cl)).await?;
    match result {
        Ok(_) => {
            debug!(
                "create_order: successfully created order for user {} for time {:?} with item_ids {:?}",
                uid, deliver_at.unwrap_or(String::from("Instant")), item_ids
            );
            Ok(HttpResponse::Ok().json(OrderResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "create_order: failed to create order for user {} for time {:?} with item_ids {:?}: {}",
                uid, deliver_at.unwrap_or(String::from("Instant")), item_ids, e
            );
            Ok(HttpResponse::Conflict().json(OrderResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Orders",
    params(
        OrderCanteenQuery,
    ),
    responses(
        (status = 200, description = "Successfully fetched aggregated counts of active ordered items", body = TimedActiveItemCountResponse)
    ),
    summary = "Get aggregated active order item counts"
)]
#[get("")]
pub(super) async fn get_all_orders(
    order_ops: web::Data<OrderOperations>,
    _admin: AdminPrincipal,
    params: web::Query<OrderCanteenQuery>,
) -> actix_web::Result<impl Responder> {
    let search_canteen_id = params.canteen_id;
    let result = web::block(move || order_ops.get_all_orders_by_count(search_canteen_id)).await?;
    match result {
        Ok(data) => {
            debug!("get_all_orders_by_count: retrieved {} items.", data.len(),);
            Ok(HttpResponse::Ok().json(TimedActiveItemCountResponse {
                status: "ok".to_string(),
                data,
                error: None,
            }))
        }
        Err(e) => Ok(
            HttpResponse::InternalServerError().json(TimedActiveItemCountResponse {
                status: "error".to_string(),
                data: TimedActiveItemCount::new(),
                error: Option::from(e.to_string()),
            }),
        ),
    }
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
    _admin: AdminPrincipal,
    order_ops: web::Data<OrderOperations>,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let search_order_id = path.into_inner().0;
    let result = order_ops.get_orders_by_orderid(&search_order_id).await;
    match result {
        Ok(data) => {
            debug!(
                "get_order_by_orderid: retrieved {} items for order_id {}",
                data.items.len(),
                search_order_id
            );
            Ok(HttpResponse::Ok().json(OrderItemsResponse {
                status: "ok".to_string(),
                data,
                error: None,
            }))
        }
        Err(e) => Ok(
            HttpResponse::InternalServerError().json(OrderItemsResponse {
                status: "error".to_string(),
                data: OrderItemContainer {
                    order_id: search_order_id,
                    total_price: 0,
                    deliver_at: String::new(),
                    items: Vec::new(),
                },
                error: Option::from(e.to_string()),
            }),
        ),
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
    user: UserPrincipal,
) -> actix_web::Result<impl Responder> {
    let search_user_id = user.user_id();
    let result = order_ops.get_orders_by_userid(&search_user_id).await;
    match result {
        Ok(data) => {
            debug!(
                "get_orders_by_user: retrieved {} orders for user_id {}",
                data.len(),
                search_user_id
            );
            Ok(HttpResponse::Ok().json(OrdersItemsResponse {
                status: "ok".to_string(),
                data,
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "get_orders_by_user: error retrieving orders for user_id {}: {}",
                search_user_id, e
            );
            Ok(
                HttpResponse::InternalServerError().json(OrdersItemsResponse {
                    status: "error".to_string(),
                    data: Vec::new(),
                    error: Some(e.to_string()),
                }),
            )
        }
    }
}

#[utoipa::path(
    tag = "Orders",
    params(
        ("id", description = "Unique identifier for the order"),
        ("action", description = "\"delivered\" for delivering and \"cancelled\" for cancelling."),
    ),
    responses(
        (status = 200, description = "Order delivered successfully", body = OrderResponse),
        (status = 409, description = "Failed to deliver order due to conflict or invalid items", body = OrderResponse)
    ),
    summary = "Deliver or cancel an existing order"
)]
#[put("/{id}/{action}")]
pub(super) async fn order_actions(
    order_ops: web::Data<OrderOperations>,
    _admin: AdminPrincipal,
    path: web::Path<(i32, String)>,
) -> actix_web::Result<impl Responder> {
    let (order_id, status) = path.into_inner();
    if !(status == "delivered" || status == "cancelled") {
        error!(
            "order_actions: failed to parse order with order_id {:?}: Invalid status: {:?}",
            order_id, status
        );
        return Ok(HttpResponse::BadRequest().json(OrderResponse {
            status: "error".to_string(),
            error: Option::from(
                format!(
                    "status cannot be {status}, must be either \"delivered\" or \"cancelled\"."
                )
                .to_string(),
            ),
        }));
    }
    let status_cl = status.clone();
    let result = web::block(move || order_ops.order_actions(&order_id, &status_cl)).await?;
    match result {
        Ok(_) => {
            debug!(
                "order_actions: successfully changed order with order_id {:?} to status {:?}",
                order_id, status
            );
            Ok(HttpResponse::Ok().json(OrderResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "order_actions: failed to change order with order_id {:?} to status {:?}: {}",
                order_id, status, e
            );
            Ok(HttpResponse::Conflict().json(OrderResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}
