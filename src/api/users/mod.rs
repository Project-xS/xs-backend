mod account;

use crate::db::UserOperations;
use account::{create_user, login};
use actix_web::middleware::NormalizePath;
use actix_web::{guard, web};
use utoipa_actix_web::{scope, service_config::ServiceConfig};

pub fn config(cfg: &mut ServiceConfig, user_ops: &UserOperations) {
    cfg.service(
        scope::scope("/auth")
            .guard(guard::Header("content-type", "application/json"))
            .app_data(web::Data::new(user_ops.clone()))
            .wrap(NormalizePath::trim())
            .service(create_user)
            .service(login),
    );
}
