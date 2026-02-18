use crate::api::common::qr::QrConfig;
use crate::api::ContentTypeHeader;
use crate::db::{HoldOperations, OrderOperations, SearchOperations};
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
pub mod qr;
mod search;

pub(super) fn config(
    cfg: &mut ServiceConfig,
    order_ops: &OrderOperations,
    hold_ops: &HoldOperations,
    search_ops: &SearchOperations,
    qr_cfg: QrConfig,
) {
    cfg.service(
        scope::scope("/orders")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(order_ops.clone()))
            .app_data(web::Data::new(hold_ops.clone()))
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
