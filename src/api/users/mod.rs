mod account;
mod orders;

use crate::api::ContentTypeHeader;
use crate::db::UserOperations;
use account::{create_user, login};
use actix_web::middleware::NormalizePath;
use actix_web::web;
use orders::get_past_orders_of_user;
use utoipa_actix_web::{scope, service_config::ServiceConfig};

pub fn config(cfg: &mut ServiceConfig, user_ops: &UserOperations) {
    cfg.service(
        scope::scope("/auth")
            .guard(ContentTypeHeader)
            .app_data(web::Data::new(user_ops.clone()))
            .wrap(NormalizePath::trim())
            .service(create_user)
            .service(login),
    )
    .service(
        scope::scope("/users")
            .app_data(web::Data::new(user_ops.clone()))
            .wrap(NormalizePath::trim())
            .service(get_past_orders_of_user),
    );
}
