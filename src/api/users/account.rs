use actix_web::{web, HttpResponse, Responder};
use crate::db::{UserOperations};
use crate::enums::users::{LoginResp, LoginReq, CreateUserResp};
use crate::models::user::{NewUser};

async fn create_user(user_ops: web::Data<UserOperations>, req_data: web::Json<NewUser>) -> impl Responder {
    let email = req_data.email.clone();
    match user_ops.create_user(req_data.into_inner()) {
        Ok(_) => {
            info!("User created: {}", email);
            HttpResponse::Ok().json(CreateUserResp { status: "ok".to_string(), error: None })
        },
        Err(e) => HttpResponse::InternalServerError().json(CreateUserResp {status: "error".to_string(), error: Some(e.to_string())})
    }
}

async fn login(user_ops: web::Data<UserOperations>, req_body: web::Json<LoginReq>) -> impl Responder {
    let email = req_body.email.clone();
    match user_ops.get_user_by_email(&req_body.email) {
        Ok(_) => {
            info!("User created: {}", email);
            HttpResponse::Ok().json(LoginResp { status: "valid".to_string(), error: None })
        },
        Err(e) => HttpResponse::Unauthorized().json(LoginResp {status: "error".to_string(), error: Some(e.to_string())})
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/login", web::post().to(login))
            .route("/create_user", web::post().to(create_user))
    );
}
