use crate::db::UserOperations;
use crate::enums::users::{CreateUserResp, LoginReq, LoginResp};
use crate::models::user::NewUser;
use actix_web::{post, put, web, HttpResponse, Responder};

#[put("/create")]
pub(super) async fn create_user(
    user_ops: web::Data<UserOperations>,
    req_data: web::Json<NewUser>,
) -> impl Responder {
    let email = req_data.email.clone();
    match user_ops.create_user(req_data.into_inner()) {
        Ok(_) => {
            info!("User created: {}", email);
            HttpResponse::Ok().json(CreateUserResp {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("ACCOUNT: create_user(): {}", e.to_string());
            HttpResponse::InternalServerError().json(CreateUserResp {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        },
    }
}

#[post("/login")]
pub(super) async fn login(
    user_ops: web::Data<UserOperations>,
    req_body: web::Json<LoginReq>,
) -> impl Responder {
    let email = req_body.email.clone();
    match user_ops.get_user_by_email(&email) {
        Ok(_) => {
            info!("User logged in: {}", email);
            HttpResponse::Ok().json(LoginResp {
                status: "valid".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("ACCOUNT: login(): {}", e.to_string());
            HttpResponse::Unauthorized().json(LoginResp {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        },
    }
}
