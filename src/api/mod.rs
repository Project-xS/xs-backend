pub mod admin;
mod errors;
pub mod users;

use actix_web::{get, web, HttpResponse, Responder};
pub(crate) use errors::default_error_handler;
use crate::AppState;

#[get("/")]
async fn root_endpoint() -> impl Responder {
    HttpResponse::Ok().body("Server up!")
}

pub(crate) fn configure(cfg: &mut web::ServiceConfig, state: &AppState) {
    cfg.service(root_endpoint)
        .configure(|cfg| admin::config(cfg, &state.menu_ops, &state.canteen_ops))
        .configure(|cfg| users::config(cfg, &state.user_ops));
}
