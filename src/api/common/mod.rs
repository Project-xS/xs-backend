use crate::api::common::qr::QrConfig;
use crate::api::ContentTypeHeader;
use crate::db::{HoldOperations, OrderOperations, PaymentOperations, SearchOperations};
use crate::services::phonepe::PhonePeClient;
use crate::sse::SseBroker;
use actix_web::middleware::NormalizePath;
use actix_web::web;
use hold::*;
use orders::*;
use qr::*;
use search::*;
use utoipa_actix_web::scope;
use utoipa_actix_web::service_config::ServiceConfig;

mod hold;
mod orders;
mod payments;
pub mod qr;
mod search;

#[allow(clippy::too_many_arguments)]
pub(super) fn config(
    cfg: &mut ServiceConfig,
    order_ops: &OrderOperations,
    hold_ops: &HoldOperations,
    payment_ops: &PaymentOperations,
    search_ops: &SearchOperations,
    sse_broker: &SseBroker,
    phonepe_client: &PhonePeClient,
    qr_cfg: QrConfig,
) {
    cfg.service(
        scope::scope("/orders")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(order_ops.clone()))
            .app_data(web::Data::new(hold_ops.clone()))
            .app_data(web::Data::new(sse_broker.clone()))
            .app_data(web::Data::new(qr_cfg))
            .service(
                scope::scope("/hold")
                    .service(
                        scope::scope("")
                            .guard(ContentTypeHeader)
                            .service(hold_order),
                    )
                    .service(confirm_hold)
                    .service(cancel_hold),
            )
            .service(generate_order_qr)
            .service(
                scope::scope("")
                    .guard(ContentTypeHeader)
                    .service(scan_order_qr),
            )
            .service(get_all_orders)
            .service(get_orders_by_user)
            .service(get_order_by_orderid)
            .service(order_actions),
    )
    .service(
        scope::scope("/payments")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(hold_ops.clone()))
            .app_data(web::Data::new(payment_ops.clone()))
            .app_data(web::Data::new(sse_broker.clone()))
            .app_data(web::Data::new(phonepe_client.clone()))
            .service(
                scope::scope("")
                    .guard(ContentTypeHeader)
                    .service(payments::initiate_payment)
                    .service(payments::verify_payment)
                    .service(payments::webhook_payment),
            ),
    )
    // Search Routes
    .service(
        scope::scope("/search")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(search_ops.clone()))
            .service(
                scope::scope("")
                    .service(get_search_query_results)
                    .service(search_query_by_canteen),
            ),
    );
}
