use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct LoginReq {
    pub email: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResp {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateUserResp {
    pub status: String,
    pub error: Option<String>,
}
