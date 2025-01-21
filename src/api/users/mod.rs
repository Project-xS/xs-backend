mod account;

use crate::db::UserOperations;
use account::{create_user, login};
use actix_web::web;
use utoipa_actix_web::{service_config::ServiceConfig, scope};

pub fn config(cfg: &mut ServiceConfig, user_ops: &UserOperations) {
    cfg.service(
        scope::scope("/auth")
            .app_data(web::Data::new(user_ops.clone()))
            .service(create_user)
            .service(login)
    );
}
