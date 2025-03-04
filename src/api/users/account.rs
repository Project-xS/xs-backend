use log::{debug, error};
use crate::db::UserOperations;
use crate::enums::users::{CreateUserResp, LoginReq, LoginResp};
use crate::models::user::NewUser;
use actix_web::{post, web, HttpResponse, Responder};

#[utoipa::path(
    tag = "User",
    request_body = NewUser,
    responses(
        (status = 200, description = "User account successfully created", body = CreateUserResp),
        (status = 409, description = "Failed to create user account", body = CreateUserResp)
    ),
    summary = "Register a new user account"
)]
#[post("/create")]
pub(super) async fn create_user(
    user_ops: web::Data<UserOperations>,
    req_data: web::Json<NewUser>,
) -> impl Responder {
    let email = req_data.email.clone();
    match user_ops.create_user(req_data.into_inner()) {
        Ok(_) => {
            debug!("create_user: successfully created user account with email '{}'", email);
            HttpResponse::Ok().json(CreateUserResp {
                status: "ok".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("create_user: failed to create user account for email '{}': {}", email, e);
            HttpResponse::Conflict().json(CreateUserResp {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}

#[utoipa::path(
    tag = "User",
    responses(
        (status = 200, description = "User authenticated successfully", body = LoginResp),
        (status = 400, description = "Authentication failed: invalid credentials or user not found", body = LoginResp)
    ),
    summary = "Authenticate a user account"
)]
#[post("/login")]
pub(super) async fn login(
    user_ops: web::Data<UserOperations>,
    req_body: web::Json<LoginReq>,
) -> impl Responder {
    let email = req_body.email.clone();
    match user_ops.get_user_by_email(&email) {
        Ok(_) => {
            debug!("login: user authenticated successfully for email '{}'", email);
            HttpResponse::Ok().json(LoginResp {
                status: "valid".to_string(),
                error: None,
            })
        }
        Err(e) => {
            error!("login: authentication failed for email '{}': {}", email, e);
            HttpResponse::BadRequest().json(LoginResp {
                status: "error".to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}
