use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

pub type TimedActiveItemCount = HashMap<String, Vec<ActiveItemCount>>;

#[derive(Serialize, ToSchema)]
pub struct ActiveItemCount {
    pub item_id: i32,
    pub item_name: String,
    pub num_ordered: i64,
}

#[derive(Serialize, ToSchema)]
pub struct TimedActiveItemCountResponse {
    pub status: String,
    pub data: TimedActiveItemCount,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct ItemContainer {
    pub canteen_name: String,
    pub name: String,
    pub quantity: i16,
    pub is_veg: bool,
    pub pic_link: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct OrderItemContainer {
    pub order_id: i32,
    pub total_price: i32,
    pub deliver_at: String,
    pub items: Vec<ItemContainer>,
}

#[derive(Serialize, ToSchema)]
pub struct OrdersItemsResponse {
    pub status: String,
    pub data: Vec<OrderItemContainer>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderItemsResponse {
    pub status: String,
    pub data: OrderItemContainer,
    pub error: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct OrderRequest {
    pub user_id: i32,
    pub deliver_at: Option<String>,
    pub item_ids: Vec<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderResponse {
    pub status: String,
    pub error: Option<String>,
}
