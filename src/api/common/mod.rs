use crate::db::{OrderOperations, SearchOperations};
use actix_web::middleware::NormalizePath;
use actix_web::web;
use orders::{create_order, get_all_orders};
use utoipa_actix_web::scope;
use utoipa_actix_web::service_config::ServiceConfig;
use crate::api::common::search::get_search_query_results;
use crate::api::ContentTypeHeader;

mod orders;
mod search;

pub(super) fn config(cfg: &mut ServiceConfig, order_ops: &OrderOperations, search_ops: &SearchOperations) {
    cfg.service(
        scope::scope("/orders")
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(order_ops.clone()))
            .service(
                scope::scope("")
                    .guard(ContentTypeHeader)
                    .service(create_order),
            )
            .service(scope::scope("").service(get_all_orders)),
    )
        .service(
            scope::scope("/search")
                .wrap(NormalizePath::trim())
                .app_data(web::Data::new(search_ops.clone()))
                .service(scope::scope("").service(get_search_query_results))
        );
}
