use crate::api::ContentTypeHeader;
use crate::db::{OrderOperations, SearchOperations};
use actix_web::middleware::NormalizePath;
use actix_web::web;
use orders::*;
use search::*;
use utoipa_actix_web::scope;
use utoipa_actix_web::service_config::ServiceConfig;

mod orders;
mod search;

pub(super) fn config(
    cfg: &mut ServiceConfig,
    order_ops: &OrderOperations,
    search_ops: &SearchOperations,
) {
    cfg.service(
        scope::scope("/orders")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(order_ops.clone()))
            .service(
                scope::scope("")
                    .guard(ContentTypeHeader)
                    .service(create_order),
            )
            .service(
                scope::scope("")
                    .service(get_all_orders)
                    .service(get_orders_by_user)
                    .service(get_order_by_orderid)
                    .service(order_actions),
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
