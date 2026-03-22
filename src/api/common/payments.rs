use super::hold::{publish_cancel_hold_inventory_event, publish_confirmed_order_events};
use crate::auth::UserPrincipal;
use crate::db::{HoldOperations, PaymentOperations, RepositoryError};
use crate::enums::common::{
    InitiatePaymentRequest, InitiatePaymentResponse, VerifyPaymentRequest, VerifyPaymentResponse,
    WebhookPaymentResponse,
};
use crate::models::common::NewPaymentOrder;
use crate::services::phonepe::PhonePeClient;
use crate::sse::{SseBroker, SseEvent};
use actix_web::http::header::AUTHORIZATION;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use log::{debug, error, warn};

const PAYMENT_STATE_CREATED: &str = "CREATED";
const PAYMENT_STATE_PENDING: &str = "PENDING";
const PAYMENT_STATE_COMPLETED: &str = "COMPLETED";
const PAYMENT_STATE_FAILED: &str = "FAILED";

#[utoipa::path(
    tag = "Payments",
    request_body = InitiatePaymentRequest,
    responses(
        (status = 200, description = "PhonePe SDK order token created/reused", body = InitiatePaymentResponse),
        (status = 409, description = "Hold validation failed", body = InitiatePaymentResponse)
    ),
    summary = "Initiate PhonePe SDK payment for a held order"
)]
#[post("/initiate")]
pub(super) async fn initiate_payment(
    payment_ops: web::Data<PaymentOperations>,
    phonepe_client: web::Data<PhonePeClient>,
    user: UserPrincipal,
    req_data: web::Json<InitiatePaymentRequest>,
) -> actix_web::Result<impl Responder> {
    if let Err(e) = phonepe_client.ensure_enabled() {
        return Ok(
            HttpResponse::InternalServerError().json(InitiatePaymentResponse {
                status: "error".to_string(),
                order_id: None,
                token: None,
                merchant_id: None,
                merchant_order_id: None,
                error: Some(e),
            }),
        );
    }

    let InitiatePaymentRequest { hold_id, amount } = req_data.into_inner();
    let uid = user.user_id();

    let hold_snapshot = match payment_ops.get_hold_snapshot_for_user(hold_id, uid) {
        Ok(snapshot) => snapshot,
        Err(e) => return Ok(conflict_initiate_response(e)),
    };

    if amount != hold_snapshot.total_price {
        return Ok(HttpResponse::Conflict().json(InitiatePaymentResponse {
            status: "error".to_string(),
            order_id: None,
            token: None,
            merchant_id: None,
            merchant_order_id: None,
            error: Some("Amount mismatch with held order total.".to_string()),
        }));
    }

    let existing_mapping = match payment_ops.find_active_mapping_by_hold_id(hold_id) {
        Ok(mapping) => mapping,
        Err(e) => {
            error!(
                "initiate_payment: failed to query active mapping for hold {}: {}",
                hold_id, e
            );
            return Ok(
                HttpResponse::InternalServerError().json(InitiatePaymentResponse {
                    status: "error".to_string(),
                    order_id: None,
                    token: None,
                    merchant_id: None,
                    merchant_order_id: None,
                    error: Some("Internal server error.".to_string()),
                }),
            );
        }
    };
    if let Some(existing_mapping) = existing_mapping {
        if existing_mapping.amount != amount {
            return Ok(HttpResponse::Conflict().json(InitiatePaymentResponse {
                status: "error".to_string(),
                order_id: None,
                token: None,
                merchant_id: None,
                merchant_order_id: None,
                error: Some("Amount mismatch with existing payment mapping.".to_string()),
            }));
        }

        debug!(
            "initiate_payment: reusing payment mapping for hold {} merchant_order_id {}",
            hold_id, existing_mapping.merchant_order_id
        );
        return Ok(HttpResponse::Ok().json(InitiatePaymentResponse {
            status: "ok".to_string(),
            order_id: Some(existing_mapping.phonepe_order_id),
            token: Some(existing_mapping.sdk_token),
            merchant_id: Some(phonepe_client.config().merchant_id.clone()),
            merchant_order_id: Some(existing_mapping.merchant_order_id),
            error: None,
        }));
    }

    // TODO: What happens when hold expires? Should we cancel the order?
    // When hold expires, and is garbage collected by the time the payment is done,
    // if stock there still after hold expired, then renew hold and place order
    // if not there, show a warning that item aint available, go ahead and ask them to contact canteen
    // we cant do refunds as it reduces gateway-merchant reputation
    // TODO: however this entire system must be implemented.

    let mut expire_after = phonepe_client.config().order_expire_after_secs;
    expire_after = expire_after.min(hold_snapshot.remaining_secs);
    if expire_after <= 0 {
        return Ok(HttpResponse::Conflict().json(InitiatePaymentResponse {
            status: "error".to_string(),
            order_id: None,
            token: None,
            merchant_id: None,
            merchant_order_id: None,
            error: Some("Hold expired or does not belong to user.".to_string()),
        }));
    }

    let merchant_order_id = format!("TXN_{}_{}", hold_id, Utc::now().timestamp_millis());
    let create_result = match phonepe_client
        .create_sdk_order(&merchant_order_id, amount, expire_after)
        .await
    {
        Ok(res) => res,
        Err(e) => {
            error!(
                "initiate_payment: failed to create PhonePe order for hold {} user {}: {}",
                hold_id, uid, e
            );
            return Ok(HttpResponse::Conflict().json(InitiatePaymentResponse {
                status: "error".to_string(),
                order_id: None,
                token: None,
                merchant_id: None,
                merchant_order_id: None,
                error: Some("Failed to initiate payment order.".to_string()),
            }));
        }
    };

    let mapping = NewPaymentOrder {
        hold_id,
        user_id: uid,
        merchant_order_id: merchant_order_id.clone(),
        phonepe_order_id: create_result.phonepe_order_id.clone(),
        sdk_token: create_result.sdk_token.clone(),
        amount,
        payment_state: PAYMENT_STATE_CREATED.to_string(),
        phonepe_expires_at: Some(Utc::now() + Duration::seconds(expire_after)),
        app_order_id: None,
    };
    let stored_mapping = match payment_ops.create_mapping(mapping) {
        Ok(stored_mapping) => stored_mapping,
        Err(e) => {
            error!(
                "initiate_payment: failed to persist mapping for hold {} user {}: {}",
                hold_id, uid, e
            );
            return Ok(
                HttpResponse::InternalServerError().json(InitiatePaymentResponse {
                    status: "error".to_string(),
                    order_id: None,
                    token: None,
                    merchant_id: None,
                    merchant_order_id: None,
                    error: Some("Failed to persist payment mapping.".to_string()),
                }),
            );
        }
    };

    Ok(HttpResponse::Ok().json(InitiatePaymentResponse {
        status: "ok".to_string(),
        order_id: Some(stored_mapping.phonepe_order_id),
        token: Some(stored_mapping.sdk_token),
        merchant_id: Some(create_result.merchant_id),
        merchant_order_id: Some(stored_mapping.merchant_order_id),
        error: None,
    }))
}

#[utoipa::path(
    tag = "Payments",
    request_body = VerifyPaymentRequest,
    params(
        ("hold_id", description = "Hold ID to verify payment for"),
    ),
    responses(
        (status = 200, description = "Payment completed and hold confirmed", body = VerifyPaymentResponse),
        (status = 409, description = "Payment pending/failed or validation mismatch", body = VerifyPaymentResponse)
    ),
    summary = "Verify PhonePe payment status for hold"
)]
#[post("/verify/{hold_id}")]
pub(super) async fn verify_payment(
    payment_ops: web::Data<PaymentOperations>,
    hold_ops: web::Data<HoldOperations>,
    broker: web::Data<SseBroker>,
    phonepe_client: web::Data<PhonePeClient>,
    user: UserPrincipal,
    path: web::Path<(i32,)>,
    req_data: web::Json<VerifyPaymentRequest>,
) -> actix_web::Result<impl Responder> {
    if let Err(e) = phonepe_client.ensure_enabled() {
        return Ok(
            HttpResponse::InternalServerError().json(VerifyPaymentResponse {
                status: "error".to_string(),
                order_id: None,
                payment_state: None,
                error: Some(e),
            }),
        );
    }

    let hold_id = path.into_inner().0;
    let uid = user.user_id();
    let merchant_order_id = req_data.into_inner().merchant_order_id;

    let mapping = match payment_ops.get_mapping_for_user_verify(hold_id, uid, &merchant_order_id) {
        Ok(mapping) => mapping,
        Err(e) => {
            return Ok(HttpResponse::Conflict().json(VerifyPaymentResponse {
                status: "error".to_string(),
                order_id: None,
                payment_state: None,
                error: Some(e.to_string()),
            }));
        }
    };

    let remote_state = match phonepe_client.fetch_order_state(&merchant_order_id).await {
        Ok(state) => state,
        Err(e) => {
            error!(
                "verify_payment: failed status lookup for merchant_order_id {}: {}",
                merchant_order_id, e
            );
            return Ok(HttpResponse::Conflict().json(VerifyPaymentResponse {
                status: "error".to_string(),
                order_id: None,
                payment_state: None,
                error: Some("Unable to verify payment status from PhonePe.".to_string()),
            }));
        }
    };

    match remote_state.as_str() {
        PAYMENT_STATE_COMPLETED => {
            if mapping.payment_state == PAYMENT_STATE_COMPLETED {
                return Ok(HttpResponse::Ok().json(VerifyPaymentResponse {
                    status: "ok".to_string(),
                    order_id: mapping.app_order_id,
                    payment_state: Some(PAYMENT_STATE_COMPLETED.to_string()),
                    error: None,
                }));
            }

            let result = web::block(move || hold_ops.confirm_held_order(hold_id, uid)).await?;
            let order_id = match result {
                Ok((order_id, user_id, canteen_id, (time_band, aggregated_updates))) => {
                    publish_confirmed_order_events(
                        &broker,
                        order_id,
                        user_id,
                        canteen_id,
                        time_band,
                        aggregated_updates,
                    );
                    order_id
                }
                Err(e) => {
                    error!(
                        "verify_payment: failed to confirm hold {} for user {} after completed payment: {}",
                        hold_id, uid, e
                    );
                    return Ok(HttpResponse::Conflict().json(VerifyPaymentResponse {
                        status: "error".to_string(),
                        order_id: None,
                        payment_state: Some(PAYMENT_STATE_COMPLETED.to_string()),
                        error: Some(e.to_string()),
                    }));
                }
            };

            let _ = payment_ops.update_mapping_state(
                &merchant_order_id,
                PAYMENT_STATE_COMPLETED,
                Some(order_id),
            );
            publish_payment_update_event(
                &broker,
                uid,
                hold_id,
                &merchant_order_id,
                PAYMENT_STATE_COMPLETED,
            );

            Ok(HttpResponse::Ok().json(VerifyPaymentResponse {
                status: "ok".to_string(),
                order_id: Some(order_id),
                payment_state: Some(PAYMENT_STATE_COMPLETED.to_string()),
                error: None,
            }))
        }
        PAYMENT_STATE_PENDING => {
            let _ =
                payment_ops.update_mapping_state(&merchant_order_id, PAYMENT_STATE_PENDING, None);
            publish_payment_update_event(
                &broker,
                uid,
                hold_id,
                &merchant_order_id,
                PAYMENT_STATE_PENDING,
            );

            Ok(HttpResponse::Conflict().json(VerifyPaymentResponse {
                status: "error".to_string(),
                order_id: None,
                payment_state: Some(PAYMENT_STATE_PENDING.to_string()),
                error: Some("Payment is still pending.".to_string()),
            }))
        }
        PAYMENT_STATE_FAILED => {
            if mapping.payment_state != PAYMENT_STATE_COMPLETED {
                let cancel_result =
                    web::block(move || hold_ops.release_held_order(hold_id, uid)).await?;
                match cancel_result {
                    Ok((canteen_id, inventory_updates)) => {
                        publish_cancel_hold_inventory_event(&broker, canteen_id, inventory_updates);
                    }
                    Err(e) => {
                        warn!(
                            "verify_payment: failed to release hold {} for user {} after failed payment: {}",
                            hold_id, uid, e
                        );
                    }
                }
                let _ = payment_ops.update_mapping_state(
                    &merchant_order_id,
                    PAYMENT_STATE_FAILED,
                    None,
                );
            }
            publish_payment_update_event(
                &broker,
                uid,
                hold_id,
                &merchant_order_id,
                PAYMENT_STATE_FAILED,
            );

            Ok(HttpResponse::Conflict().json(VerifyPaymentResponse {
                status: "error".to_string(),
                order_id: None,
                payment_state: Some(PAYMENT_STATE_FAILED.to_string()),
                error: Some("Payment failed.".to_string()),
            }))
        }
        other_state => Ok(HttpResponse::Conflict().json(VerifyPaymentResponse {
            status: "error".to_string(),
            order_id: None,
            payment_state: Some(other_state.to_string()),
            error: Some(format!(
                "Unsupported payment state received from gateway: {}",
                other_state
            )),
        })),
    }
}

#[utoipa::path(
    tag = "Payments",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Webhook accepted", body = WebhookPaymentResponse),
        (status = 401, description = "Webhook authorization failed", body = WebhookPaymentResponse)
    ),
    summary = "PhonePe payment webhook callback"
)]
#[post("/webhook")]
pub(super) async fn webhook_payment(
    req: HttpRequest,
    payment_ops: web::Data<PaymentOperations>,
    hold_ops: web::Data<HoldOperations>,
    broker: web::Data<SseBroker>,
    phonepe_client: web::Data<PhonePeClient>,
    payload: web::Json<serde_json::Value>,
) -> actix_web::Result<impl Responder> {
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if !phonepe_client.verify_webhook_header(auth_header) {
        return Ok(HttpResponse::Unauthorized().json(WebhookPaymentResponse {
            status: "error".to_string(),
            error: Some("Invalid webhook authorization.".to_string()),
        }));
    }

    let body = payload.into_inner();
    let event = body
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    let merchant_order_id = extract_webhook_merchant_order_id(&body);
    let remote_state = extract_webhook_state(&body);

    let Some(merchant_order_id) = merchant_order_id else {
        return Ok(HttpResponse::BadRequest().json(WebhookPaymentResponse {
            status: "error".to_string(),
            error: Some("Webhook payload missing merchantOrderId.".to_string()),
        }));
    };

    let mapping_lookup = match payment_ops.get_mapping_by_merchant_order_id(&merchant_order_id) {
        Ok(mapping) => mapping,
        Err(e) => {
            error!(
                "webhook_payment: failed to lookup merchant_order_id {}: {}",
                merchant_order_id, e
            );
            return Ok(
                HttpResponse::InternalServerError().json(WebhookPaymentResponse {
                    status: "error".to_string(),
                    error: Some("Internal server error.".to_string()),
                }),
            );
        }
    };
    let Some(mapping) = mapping_lookup else {
        warn!(
            "webhook_payment: unknown merchant_order_id {}, acknowledging with no-op",
            merchant_order_id
        );
        return Ok(HttpResponse::Ok().json(WebhookPaymentResponse {
            status: "ok".to_string(),
            error: None,
        }));
    };

    if PaymentOperations::is_terminal_state(&mapping.payment_state) {
        debug!(
            "webhook_payment: merchant_order_id {} already terminal in state {}",
            merchant_order_id, mapping.payment_state
        );
        return Ok(HttpResponse::Ok().json(WebhookPaymentResponse {
            status: "ok".to_string(),
            error: None,
        }));
    }

    let final_state = if event == "checkout.order.completed"
        || remote_state.as_deref() == Some(PAYMENT_STATE_COMPLETED)
    {
        PAYMENT_STATE_COMPLETED
    } else if event == "checkout.order.failed"
        || remote_state.as_deref() == Some(PAYMENT_STATE_FAILED)
    {
        PAYMENT_STATE_FAILED
    } else {
        debug!(
            "webhook_payment: ignoring unsupported event '{}' for merchant_order_id {}",
            event, merchant_order_id
        );
        return Ok(HttpResponse::Ok().json(WebhookPaymentResponse {
            status: "ok".to_string(),
            error: None,
        }));
    };

    if final_state == PAYMENT_STATE_COMPLETED {
        let hold_id = mapping.hold_id;
        let user_id = mapping.user_id;
        let confirm_result =
            web::block(move || hold_ops.confirm_held_order(hold_id, user_id)).await?;
        let confirmed_order_id = match confirm_result {
            Ok((order_id, user_id, canteen_id, (time_band, aggregated_updates))) => {
                publish_confirmed_order_events(
                    &broker,
                    order_id,
                    user_id,
                    canteen_id,
                    time_band,
                    aggregated_updates,
                );
                Some(order_id)
            }
            Err(e) => {
                warn!(
                    "webhook_payment: confirmation race for merchant_order_id {} hold {}: {}",
                    merchant_order_id, mapping.hold_id, e
                );
                mapping.app_order_id
            }
        };

        let _ = payment_ops.update_mapping_state(
            &merchant_order_id,
            PAYMENT_STATE_COMPLETED,
            confirmed_order_id,
        );
        publish_payment_update_event(
            &broker,
            mapping.user_id,
            mapping.hold_id,
            &merchant_order_id,
            PAYMENT_STATE_COMPLETED,
        );
    } else {
        let hold_id = mapping.hold_id;
        let user_id = mapping.user_id;
        let release_result =
            web::block(move || hold_ops.release_held_order(hold_id, user_id)).await?;
        match release_result {
            Ok((canteen_id, inventory_updates)) => {
                publish_cancel_hold_inventory_event(&broker, canteen_id, inventory_updates);
            }
            Err(e) => {
                warn!(
                    "webhook_payment: failed/redundant release for merchant_order_id {} hold {}: {}",
                    merchant_order_id, mapping.hold_id, e
                );
            }
        }
        let _ = payment_ops.update_mapping_state(&merchant_order_id, PAYMENT_STATE_FAILED, None);
        publish_payment_update_event(
            &broker,
            mapping.user_id,
            mapping.hold_id,
            &merchant_order_id,
            PAYMENT_STATE_FAILED,
        );
    }

    Ok(HttpResponse::Ok().json(WebhookPaymentResponse {
        status: "ok".to_string(),
        error: None,
    }))
}

fn conflict_initiate_response(e: RepositoryError) -> HttpResponse {
    match e {
        RepositoryError::NotFound(_) | RepositoryError::ValidationError(_) => {
            HttpResponse::Conflict().json(InitiatePaymentResponse {
                status: "error".to_string(),
                order_id: None,
                token: None,
                merchant_id: None,
                merchant_order_id: None,
                error: Some("Hold expired or does not belong to user.".to_string()),
            })
        }
        other => {
            error!("initiate_payment: unexpected error: {}", other);
            HttpResponse::InternalServerError().json(InitiatePaymentResponse {
                status: "error".to_string(),
                order_id: None,
                token: None,
                merchant_id: None,
                merchant_order_id: None,
                error: Some("Internal server error.".to_string()),
            })
        }
    }
}

fn publish_payment_update_event(
    broker: &SseBroker,
    user_id: i32,
    hold_id: i32,
    merchant_order_id: &str,
    state: &str,
) {
    broker.publish_user_event(
        user_id,
        &SseEvent::PaymentUpdate {
            hold_id,
            merchant_order_id: merchant_order_id.to_string(),
            payment_state: state.to_string(),
        },
    );
}

fn extract_webhook_merchant_order_id(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/payload/merchantOrderId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            value
                .get("merchantOrderId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
}

fn extract_webhook_state(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/payload/state")
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase())
        .or_else(|| {
            value
                .get("state")
                .and_then(|v| v.as_str())
                .map(|s| s.to_uppercase())
        })
}
