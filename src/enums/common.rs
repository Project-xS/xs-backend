use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ActiveItemCount {
    pub item_id: i32,
    pub num_ordered: i32
}

#[derive(Serialize, ToSchema)]
pub struct ActiveItemCountResponse {
    pub status: String,
    pub data: Vec<ActiveItemCount>,
    pub error: Option<String>,
}
