use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String
}

#[derive(Serialize)]
pub struct LoginResp {
    pub status: String,
    pub error: Option<String>
}

#[derive(Serialize)]
pub struct CreateUserResp {
    pub status: String,
    pub error: Option<String>
}