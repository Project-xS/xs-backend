mod events;
mod orders;

use crate::api::users::events::user_order_events;
use crate::db::UserOperations;
use crate::sse::SseBroker;
use actix_web::middleware::NormalizePath;
use actix_web::web;
use orders::get_past_orders_of_user;
use utoipa_actix_web::{scope, service_config::ServiceConfig};

pub fn config(cfg: &mut ServiceConfig, user_ops: &UserOperations, sse_broker: &SseBroker) {
    cfg.service(
        scope::scope("/users")
            .service(
                scope::scope("/events")
                    .app_data(web::Data::new(sse_broker.clone()))
                    .wrap(NormalizePath::trim())
                    .service(user_order_events),
            )
            .service(
                scope::scope("")
                    .app_data(web::Data::new(user_ops.clone()))
                    .wrap(NormalizePath::trim())
                    .service(get_past_orders_of_user),
            ),
    );
}
