use crate::auth::extractors::AdminPrincipal;
use crate::auth::qr_token;
use crate::auth::UserPrincipal;
use crate::db::OrderOperations;
use crate::enums::common::{OrderItemsResponse, ScanQrRequest, ScanQrResponse};
use actix_web::{get, post, web, HttpResponse, Responder};
use image::ImageEncoder;
use log::{debug, error};
use qrcode::QrCode;

/// Shared config for QR token operations, injected via web::Data.
pub struct QrConfig {
    pub secret: String,
    pub max_age_secs: u64,
}

#[utoipa::path(
    tag = "Orders",
    params(
        ("id", description = "Order ID to generate QR for"),
    ),
    responses(
        (status = 200, description = "QR code PNG image", content_type = "image/png"),
        (status = 403, description = "Not your order"),
        (status = 500, description = "Failed to generate QR code")
    ),
    summary = "Generate a QR code for an active order"
)]
#[get("/{id}/qr")]
pub(super) async fn generate_order_qr(
    order_ops: web::Data<OrderOperations>,
    qr_cfg: web::Data<QrConfig>,
    user: UserPrincipal,
    path: web::Path<(i32,)>,
) -> actix_web::Result<impl Responder> {
    let order_id = path.into_inner().0;
    let uid = user.user_id();

    // Verify the order exists and belongs to this user
    let order_data = order_ops.get_orders_by_orderid_no_pics(&order_id).await;
    match order_data {
        Ok(Some(ref data)) if data.items.is_empty() => {
            return Ok(HttpResponse::NotFound().json(OrderItemsResponse {
                status: "error".to_string(),
                data: None,
                error: Some("Order not found".to_string()),
            }));
        }
        Ok(None) => {
            return Ok(HttpResponse::NotFound().json(OrderItemsResponse {
                status: "error".to_string(),
                data: None,
                error: Some("Order not found".to_string()),
            }));
        }
        Err(e) => {
            error!(
                "generate_order_qr: error fetching order {}: {}",
                order_id, e
            );
            return Ok(
                HttpResponse::InternalServerError().json(OrderItemsResponse {
                    status: "error".to_string(),
                    data: None,
                    error: Some(e.to_string()),
                }),
            );
        }
        Ok(Some(_)) => {}
    }

    // Verify ownership by checking if user has this order
    let user_orders = order_ops.get_orders_by_userid(&uid).await.map_err(|e| {
        error!(
            "generate_order_qr: error verifying ownership for user {} order {}: {}",
            uid, order_id, e
        );
        actix_web::error::ErrorInternalServerError("Failed to verify order ownership")
    })?;

    let owns_order = user_orders.iter().any(|o| o.order_id == order_id);
    if !owns_order {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "status": "error",
            "error": "You do not own this order"
        })));
    }

    // Generate token
    let token = qr_token::generate_qr_token(order_id, uid, &qr_cfg.secret);

    // Generate QR code PNG
    let qr = QrCode::new(token.as_bytes()).map_err(|e| {
        error!("generate_order_qr: QR encoding error: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to generate QR code")
    })?;

    let image = qr.render::<image::Luma<u8>>().quiet_zone(true).build();

    let mut png_buf: Vec<u8> = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::L8,
        )
        .map_err(|e| {
            error!("generate_order_qr: PNG encoding error: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to encode QR image")
        })?;

    debug!(
        "generate_order_qr: generated QR for order {} user {}",
        order_id, uid
    );

    Ok(HttpResponse::Ok().content_type("image/png").body(png_buf))
}

#[utoipa::path(
    tag = "Orders",
    request_body = ScanQrRequest,
    responses(
        (status = 200, description = "Order details from QR scan", body = ScanQrResponse),
        (status = 400, description = "Invalid or expired QR token", body = ScanQrResponse),
        (status = 403, description = "Valid QR but order is for a different canteen", body = ScanQrResponse),
    ),
    summary = "Scan a QR code to retrieve order details (merchant only)"
)]
#[post("/scan")]
pub(super) async fn scan_order_qr(
    order_ops: web::Data<OrderOperations>,
    qr_cfg: web::Data<QrConfig>,
    admin: AdminPrincipal,
    req_data: web::Json<ScanQrRequest>,
) -> actix_web::Result<impl Responder> {
    let token = &req_data.token;

    let (order_id, _user_id) =
        match qr_token::verify_qr_token(token, &qr_cfg.secret, qr_cfg.max_age_secs) {
            Ok(result) => result,
            Err(e) => {
                debug!("scan_order_qr: token verification failed: {}", e);
                return Ok(HttpResponse::BadRequest().json(ScanQrResponse {
                    status: "error".to_string(),
                    data: None,
                    error: Some(e),
                }));
            }
        };

    // Fetch order details
    let order_data = order_ops.get_orders_by_orderid_no_pics(&order_id).await;
    match order_data {
        Ok(Some(data)) => {
            if data.items.is_empty() {
                debug!("scan_order_qr: order {} not found or empty", order_id);
                return Ok(HttpResponse::BadRequest().json(ScanQrResponse {
                    status: "error".to_string(),
                    data: None,
                    error: Some("Order not found or already completed".to_string()),
                }));
            }
            let order_canteen_id = data.items.first().map(|item| item.canteen_id).unwrap_or(0);
            if order_canteen_id != admin.canteen_id {
                debug!(
                    "scan_order_qr: order {} belongs to canteen {}, not admin canteen {}",
                    order_id, order_canteen_id, admin.canteen_id
                );
                return Ok(HttpResponse::Forbidden().json(ScanQrResponse {
                    status: "error".to_string(),
                    data: None,
                    error: Some("QR is valid but does not belong to this shop's order".to_string()),
                }));
            }
            debug!(
                "scan_order_qr: retrieved order {} with {} items",
                order_id,
                data.items.len()
            );
            Ok(HttpResponse::Ok().json(ScanQrResponse {
                status: "ok".to_string(),
                data: Some(data),
                error: None,
            }))
        }
        Ok(None) => {
            debug!("scan_order_qr: order {} not found", order_id);
            Ok(HttpResponse::BadRequest().json(ScanQrResponse {
                status: "error".to_string(),
                data: None,
                error: Some("Order not found or already completed".to_string()),
            }))
        }
        Err(e) => {
            error!("scan_order_qr: error fetching order {}: {}", order_id, e);
            Ok(HttpResponse::InternalServerError().json(ScanQrResponse {
                status: "error".to_string(),
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}
