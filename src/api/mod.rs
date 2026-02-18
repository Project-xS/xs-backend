pub mod admin;
pub mod common;
pub mod errors;
pub mod users;

use crate::AppState;
use actix_web::guard::{Guard, GuardContext};
use actix_web::{get, HttpResponse, Responder};
use common::qr::QrConfig;
pub use errors::default_error_handler;
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

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check successful")
    ),
    summary = "Health check"
)]
#[get("/health")]
async fn health_endpoint() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

pub struct ContentTypeHeader;

// Hacky route to account for utf-8 on header for flutteer
impl Guard for ContentTypeHeader {
    fn check(&self, req: &GuardContext) -> bool {
        req.head()
            .headers()
            .get(actix_web::http::header::CONTENT_TYPE)
            .and_then(|hv| hv.to_str().ok())
            .map(|ct| {
                matches!(
                    ct.to_lowercase().trim(),
                    "application/json" | "application/json; charset=utf-8"
                )
            })
            .unwrap_or(false)
    }
}

pub fn configure(cfg: &mut ServiceConfig, state: &AppState, qr_cfg: QrConfig) {
    cfg.service(root_endpoint)
        .service(health_endpoint)
        .configure(|cfg| admin::config(cfg, &state.menu_ops, &state.canteen_ops, &state.asset_ops))
        .configure(|cfg| users::config(cfg, &state.user_ops))
        .configure(|cfg| {
            common::config(
                cfg,
                &state.order_ops,
                &state.hold_ops,
                &state.search_ops,
                qr_cfg,
            )
        });
}
