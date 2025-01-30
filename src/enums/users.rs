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

#[derive(Deserialize, ToSchema)]
pub struct OrderRequest {
    pub user_id: i32,
    pub item_ids: Vec<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderResponse {
    pub status: String,
    pub error: Option<String>,
}
