use crate::auth::extractors::PrincipalExtractor;
use crate::auth::{AdminPrincipal, Principal};
use crate::db::OrderOperations;
use crate::enums::common::{
    OrderItemsResponse, OrderResponse, OrdersItemsResponse, TimedActiveItemCount,
    TimedActiveItemCountResponse,
};
use actix_web::{get, put, web, HttpResponse, Responder};
use log::{debug, error};
use serde::Deserialize;
use utoipa::IntoParams;

#[utoipa::path(
    tag = "Orders",
    responses(
        (status = 200, description = "Successfully fetched aggregated counts of active ordered items", body = TimedActiveItemCountResponse)
    ),
    summary = "Get aggregated active order item counts"
)]
#[get("")]
pub(super) async fn get_all_orders(
    order_ops: web::Data<OrderOperations>,
    admin: AdminPrincipal,
) -> actix_web::Result<impl Responder> {
    let search_canteen_id = admin.canteen_id;
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
            if let Some(ref order) = data {
                debug!(
                    "get_order_by_orderid: retrieved {} items for order_id {}",
                    order.items.len(),
                    search_order_id
                );
            } else {
                debug!(
                    "get_order_by_orderid: no active order found for order_id {}",
                    search_order_id
                );
            }
            Ok(HttpResponse::Ok().json(OrderItemsResponse {
                status: "ok".to_string(),
                data,
                error: None,
            }))
        }
        Err(e) => Ok(
            HttpResponse::InternalServerError().json(OrderItemsResponse {
                status: "error".to_string(),
                data: None,
                error: Option::from(e.to_string()),
            }),
        ),
    }
}

#[derive(Deserialize, Debug, IntoParams)]
struct UserOrderQuery {
    user_id: Option<i32>,
    rfid: Option<String>,
}

#[utoipa::path(
    tag = "Orders",
    params(
        UserOrderQuery,
    ),
    responses(
        (status = 200, description = "Successfully retrieved order items for the specified user or RFID", body = OrdersItemsResponse),
        (status = 500, description = "Failed to retrieve order items due to server error", body = OrdersItemsResponse)
    ),
    summary = "Get active order items for a specified user or RFID",
    description = "Users (Firebase) fetch their own orders (no query params). Admins must provide exactly one of user_id or rfid."
)]
#[get("/by_user")]
pub(super) async fn get_orders_by_user(
    order_ops: web::Data<OrderOperations>,
    principal: PrincipalExtractor,
    params: web::Query<UserOrderQuery>,
) -> actix_web::Result<impl Responder> {
    let params = params.into_inner();

    match principal.0 {
        Principal::Admin { .. } => {
            if params.user_id.is_some() && params.rfid.is_some() {
                return Ok(HttpResponse::BadRequest().json(OrdersItemsResponse {
                    status: "error".to_string(),
                    error: Some("Cannot provide both user_id and rfid parameters".to_string()),
                    data: None,
                }));
            }

            if let Some(search_user_id) = params.user_id {
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
                            data: Some(data),
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
                                data: None,
                                error: Some(e.to_string()),
                            }),
                        )
                    }
                }
            } else if let Some(search_rfid) = params.rfid {
                let result = order_ops.get_orders_by_rfid(&search_rfid).await;
                match result {
                    Ok(data) => {
                        debug!(
                            "get_orders_by_user: retrieved {} orders for rfid '{}'",
                            data.len(),
                            search_rfid
                        );
                        Ok(HttpResponse::Ok().json(OrdersItemsResponse {
                            status: "ok".to_string(),
                            data: Some(data),
                            error: None,
                        }))
                    }
                    Err(e) => {
                        error!(
                            "get_orders_by_user: error retrieving orders for rfid '{}': {}",
                            search_rfid, e
                        );
                        Ok(
                            HttpResponse::InternalServerError().json(OrdersItemsResponse {
                                status: "error".to_string(),
                                data: None,
                                error: Some(e.to_string()),
                            }),
                        )
                    }
                }
            } else {
                Ok(HttpResponse::BadRequest().json(OrdersItemsResponse {
                    status: "error".to_string(),
                    error: Some("Either user_id or rfid must be provided".to_string()),
                    data: None,
                }))
            }
        }
        Principal::User { user_id, .. } => {
            let result = order_ops.get_orders_by_userid(&user_id).await;
            match result {
                Ok(data) => {
                    debug!(
                        "get_orders_by_user: retrieved {} orders for user_id {}",
                        data.len(),
                        user_id
                    );
                    Ok(HttpResponse::Ok().json(OrdersItemsResponse {
                        status: "ok".to_string(),
                        data: Some(data),
                        error: None,
                    }))
                }
                Err(e) => {
                    error!(
                        "get_orders_by_user: error retrieving orders for user_id {}: {}",
                        user_id, e
                    );
                    Ok(
                        HttpResponse::InternalServerError().json(OrdersItemsResponse {
                            status: "error".to_string(),
                            data: None,
                            error: Some(e.to_string()),
                        }),
                    )
                }
            }
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
