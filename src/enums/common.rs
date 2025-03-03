use serde::Serialize;
use utoipa::ToSchema;
use crate::models::common::OrderItems;

#[derive(Serialize, ToSchema)]
pub struct ActiveItemCount {
    pub item_id: i32,
    pub num_ordered: i32,
}

#[derive(Serialize, ToSchema)]
pub struct ActiveItemCountResponse {
    pub status: String,
    pub data: Vec<ActiveItemCount>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OrdersResponse {
    pub status: String,
    pub data: Vec<OrderItems>,
    pub error: Option<String>,
}
