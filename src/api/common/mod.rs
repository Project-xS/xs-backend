use actix_web::{guard, web};
use actix_web::middleware::NormalizePath;
use utoipa_actix_web::scope;
use utoipa_actix_web::service_config::ServiceConfig;
use orders::{get_all_orders, create_order};
use crate::db::{OrderOperations};

mod orders;

pub(super) fn config(
    cfg: &mut ServiceConfig,
    order_ops: &OrderOperations,
) {
    cfg.service(
        scope::scope("/orders")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(order_ops.clone()))
            .service(
                scope::scope("")
                    .guard(guard::Header("content-type", "application/json"))
                    .service(create_order)
            )
            .service(
                scope::scope("")
                    .service(get_all_orders)
            )
    );
}
