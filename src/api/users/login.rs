use actix_web::{web, HttpResponse, Responder};
use crate::db::UserOperations;
use crate::enums::users::{LoginResp, LoginReq};

async fn login(user_ops: web::Data<UserOperations>, req_body: web::Json<LoginReq>) -> impl Responder {
    match user_ops.get_user_by_email(&req_body.email) {
        Ok(_) => HttpResponse::Ok().json(LoginResp {status: "valid".to_string(), error: None }),
        Err(e) => HttpResponse::Unauthorized().json(LoginResp {status: "error".to_string(), error: Some(e.to_string())})
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
    );
}