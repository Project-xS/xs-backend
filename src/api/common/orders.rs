use crate::db::OrderOperations;
use crate::enums::common::{ActiveItemCountResponse, OrderItemContainer, OrderItemsResponse, OrdersItemsResponse};
use crate::enums::users::{OrderRequest, OrderResponse};
use actix_web::{get, post, web, HttpResponse, Responder};
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Deserialize, Debug, IntoParams)]
struct UserOrderQuery {
    user_id: Option<i32>,
    rfid: Option<String>,
}

#[utoipa::path(
    post,
    tag = "Orders",
    path = "",
    request_body = OrderRequest,
    responses(
        (status = 200, description = "Order created successfully", body = OrderResponse),
        (status = 409, description = "Order cannot be created", body = OrderResponse)
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
            HttpResponse::Conflict().json(OrderResponse {
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
        (status = 200, description = "Orders to count fetched", body = ActiveItemCountResponse)
    ),
    summary = "Returns order -> count"
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
        ("id", description = "Unique id of the order"),
    ),
    responses(
        (status = 200, description = "The order items in an order", body = ActiveItemCountResponse)
    ),
    summary = "Returns the items in an order"
)]
#[get("/{id}")]
pub(super) async fn get_order_by_orderid(order_ops: web::Data<OrderOperations>, path: web::Path<(i32,)>,) -> impl Responder {
    let search_order_id = path.into_inner().0;
    match order_ops.get_orders_by_orderid(&search_order_id) {
        Ok(data) => HttpResponse::Ok().json(OrderItemsResponse {
            status: "ok".to_string(),
            data,
            error: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(OrderItemsResponse {
            status: "error".to_string(),
            data: OrderItemContainer {
                order_id: search_order_id,
                items: Vec::new(),
            },
            error: Option::from(e.to_string()),
        })
    }
}

#[utoipa::path(
    tag = "Orders",
    params(
        UserOrderQuery,
    ),
    responses(
        (status = 200, description = "Menu items in all active orders of a user", body = OrderItemsResponse),
        (status = 500, description = "Error", body = OrderItemsResponse)
    ),
    summary = "Returns order items involved in all active orders of a specified user or rfid."
)]
#[get("/by_user")]
pub(super) async fn get_orders_by_user(order_ops: web::Data<OrderOperations>, params: web::Query<UserOrderQuery>) -> impl Responder {
    if params.user_id.is_some() && params.rfid.is_some() {
        return HttpResponse::BadRequest().json(OrdersItemsResponse {
            status: "error".to_string(),
            error: Option::from("Cannot provide both user_id and rfid parameters".to_string()),
            data: Vec::new()
        });
    }
    if let Some(search_user_id) = &params.user_id {
        match order_ops.get_orders_by_userid(search_user_id) {
            Ok(data) => HttpResponse::Ok().json(OrdersItemsResponse {
                status: "ok".to_string(),
                data,
                error: None,
            }),
            Err(e) => HttpResponse::InternalServerError().json(OrdersItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string())
            })
        }
    }
    else if let Some(search_rfid) = &params.rfid {
        match order_ops.get_orders_by_rfid(search_rfid) {
            Ok(data) => HttpResponse::Ok().json(OrdersItemsResponse {
                status: "ok".to_string(),
                data,
                error: None,
            }),
            Err(e) => HttpResponse::InternalServerError().json(OrdersItemsResponse {
                status: "error".to_string(),
                data: Vec::new(),
                error: Some(e.to_string())
            })
        }
    }
    else {
        HttpResponse::BadRequest().json(OrdersItemsResponse {
            status: "error".to_string(),
            error: Option::from("Either user_id or rfid must be provided".to_string()),
            data: Vec::new()
        })
    }
}
