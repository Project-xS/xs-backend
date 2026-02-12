use crate::auth::UserPrincipal;
use crate::db::HoldOperations;
use crate::enums::common::{ConfirmHoldResponse, HoldOrderResponse, OrderRequest, OrderResponse};
use actix_web::{delete, post, web, HttpResponse, Responder};
use log::{debug, error};

#[utoipa::path(
    tag = "Orders",
    request_body = OrderRequest,
    responses(
        (status = 200, description = "Order held successfully, stock reserved", body = HoldOrderResponse),
        (status = 409, description = "Failed to hold order due to stock/validation issues", body = HoldOrderResponse)
    ),
    summary = "Hold (reserve) an order for payment"
)]
#[post("")]
pub(super) async fn hold_order(
    hold_ops: web::Data<HoldOperations>,
    user: UserPrincipal,
    req_data: web::Json<OrderRequest>,
) -> actix_web::Result<impl Responder> {
    let OrderRequest {
        deliver_at,
        item_ids,
    } = req_data.into_inner();

    if deliver_at.is_some()
        && (deliver_at != Some(String::from("11:00am - 12:00pm"))
            && deliver_at != Some(String::from("12:00pm - 01:00pm")))
    {
        return Ok(HttpResponse::BadRequest().json(HoldOrderResponse {
            status: "error".to_string(),
            hold_id: None,
            expires_at: None,
            error: Some(format!(
                "Invalid time band: {}",
                deliver_at.unwrap_or_default()
            )),
        }));
    }

    let uid = user.user_id();
    let deliver_at_cl = deliver_at.clone();
    let item_ids_cl = item_ids.clone();
    let result = web::block(move || hold_ops.hold_order(uid, item_ids_cl, deliver_at_cl)).await?;

    match result {
        Ok((hold_id, expires_at)) => {
            debug!(
                "hold_order: created hold {} for user {} with items {:?}",
                hold_id, uid, item_ids
            );
            Ok(HttpResponse::Ok().json(HoldOrderResponse {
                status: "ok".to_string(),
                hold_id: Some(hold_id),
                expires_at: Some(expires_at),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "hold_order: failed to hold order for user {} with items {:?}: {}",
                uid, item_ids, e
            );
            Ok(HttpResponse::Conflict().json(HoldOrderResponse {
                status: "error".to_string(),
                hold_id: None,
                expires_at: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Orders",
    params(
        ("id", description = "Hold ID to confirm"),
    ),
    responses(
        (status = 200, description = "Hold confirmed, order created", body = ConfirmHoldResponse),
        (status = 409, description = "Failed to confirm hold", body = ConfirmHoldResponse)
    ),
    summary = "Confirm a held order after payment"
)]
#[post("/{id}/confirm")]
pub(super) async fn confirm_hold(
    hold_ops: web::Data<HoldOperations>,
    user: UserPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let hold_id = path.into_inner().0;
    let uid = user.user_id();
    let result = web::block(move || hold_ops.confirm_held_order(hold_id, uid)).await?;

    match result {
        Ok(order_id) => {
            debug!(
                "confirm_hold: hold {} confirmed as order {} for user {}",
                hold_id, order_id, uid
            );
            Ok(HttpResponse::Ok().json(ConfirmHoldResponse {
                status: "ok".to_string(),
                order_id: Some(order_id),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "confirm_hold: failed to confirm hold {} for user {}: {}",
                hold_id, uid, e
            );
            Ok(HttpResponse::Conflict().json(ConfirmHoldResponse {
                status: "error".to_string(),
                order_id: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

#[utoipa::path(
    tag = "Orders",
    params(
        ("id", description = "Hold ID to cancel"),
    ),
    responses(
        (status = 200, description = "Hold cancelled, stock released", body = OrderResponse),
        (status = 409, description = "Failed to cancel hold", body = OrderResponse)
    ),
    summary = "Cancel a held order and release reserved stock"
)]
#[delete("/{id}")]
pub(super) async fn cancel_hold(
    hold_ops: web::Data<HoldOperations>,
    user: UserPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let hold_id = path.into_inner().0;
    let uid = user.user_id();
    let result = web::block(move || hold_ops.release_held_order(hold_id, uid)).await?;

    match result {
        Ok(()) => {
            debug!("cancel_hold: hold {} cancelled for user {}", hold_id, uid);
            Ok(HttpResponse::Ok().json(OrderResponse {
                status: "ok".to_string(),
                error: None,
            }))
        }
        Err(e) => {
            error!(
                "cancel_hold: failed to cancel hold {} for user {}: {}",
                hold_id, uid, e
            );
            Ok(HttpResponse::Conflict().json(OrderResponse {
                status: "error".to_string(),
                error: Some(e.to_string()),
            }))
        }
    }
}
