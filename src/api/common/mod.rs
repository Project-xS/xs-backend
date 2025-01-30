use crate::db::OrderOperations;
use actix_web::middleware::NormalizePath;
use actix_web::{guard, web};
use orders::{create_order, get_all_orders};
use utoipa_actix_web::scope;
use utoipa_actix_web::service_config::ServiceConfig;

mod orders;

pub(super) fn config(cfg: &mut ServiceConfig, order_ops: &OrderOperations) {
    cfg.service(
        scope::scope("/orders")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(order_ops.clone()))
            .service(
                scope::scope("")
                    .guard(guard::Header("content-type", "application/json"))
                    .service(create_order),
            )
            .service(scope::scope("").service(get_all_orders)),
    );
}
