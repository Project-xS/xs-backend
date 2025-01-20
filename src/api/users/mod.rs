mod account;

use crate::db::UserOperations;
use account::{create_user, login};
use actix_web::web;
use actix_web::web::Data;

pub fn config(cfg: &mut web::ServiceConfig, user_ops: &UserOperations) {
    cfg.service(
        web::scope("/auth")
            .app_data(Data::new(user_ops.clone()))
            .service(create_user)
            .service(login)
    );
}
