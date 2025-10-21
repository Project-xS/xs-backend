mod orders;

use crate::db::UserOperations;
use actix_web::middleware::NormalizePath;
use actix_web::web;
use orders::get_past_orders_of_user;
use utoipa_actix_web::{scope, service_config::ServiceConfig};

pub fn config(cfg: &mut ServiceConfig, user_ops: &UserOperations) {
    cfg.service(
        scope::scope("/users")
            .app_data(web::Data::new(user_ops.clone()))
            .wrap(NormalizePath::trim())
            .service(get_past_orders_of_user),
    );
}
