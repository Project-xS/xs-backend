mod admin;
mod common;
mod errors;
mod users;

use crate::AppState;
use actix_web::{get, HttpResponse, Responder};
pub(crate) use errors::default_error_handler;
use utoipa_actix_web::service_config::ServiceConfig;

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Health check successful")
    ),
    summary = "Is the server up?"
)]
#[get("/")]
async fn root_endpoint() -> impl Responder {
    HttpResponse::Ok().body("Server up!")
}

pub(crate) fn configure(cfg: &mut ServiceConfig, state: &AppState) {
    cfg.service(root_endpoint)
        .configure(|cfg| admin::config(cfg, &state.menu_ops, &state.canteen_ops))
        .configure(|cfg| users::config(cfg, &state.user_ops))
        .configure(|cfg| common::config(cfg, &state.order_ops, &state.search_ops));
}
