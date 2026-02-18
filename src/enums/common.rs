use crate::models::common::{OrderItems, TimeBandEnum};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use with_pic_macro::{with_pic, WithPic};

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
    pub name: String,
    pub quantity: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<i32>,
    pub is_veg: bool,
    pub pic_link: Option<String>,
    pub pic_etag: Option<String>,
    pub description: Option<String>,
}

#[with_pic(OrderItems)]
#[derive(Serialize, Debug, WithPic)]
pub struct OrderItemsWithPic {
    pub order_id: i32,
    pub canteen_name: String,
    pub item_id: i32,
    pub total_price: i32,
    pub deliver_at: Option<TimeBandEnum>,
    pub ordered_at: DateTime<Utc>,
    pub name: String,
    pub quantity: i16,
    pub price: i32,
    pub is_veg: bool,
    pub pic_link: Option<String>,
    pub pic_etag: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct OrderItemContainer {
    pub order_id: i32,
    pub canteen_name: String,
    pub total_price: i32,
    pub deliver_at: String,
    pub ordered_at: i64,
    pub items: Vec<ItemContainer>,
}

#[derive(Serialize, ToSchema)]
pub struct OrdersItemsResponse {
    pub status: String,
    pub data: Option<Vec<OrderItemContainer>>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderItemsResponse {
    pub status: String,
    pub data: Option<OrderItemContainer>,
    pub error: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct OrderRequest {
    pub deliver_at: Option<String>,
    pub item_ids: Vec<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderResponse {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct HoldOrderResponse {
    pub status: String,
    pub hold_id: Option<i32>,
    pub expires_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ConfirmHoldResponse {
    pub status: String,
    pub order_id: Option<i32>,
    pub error: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ScanQrRequest {
    pub token: String,
}

#[derive(Serialize, ToSchema)]
pub struct ScanQrResponse {
    pub status: String,
    pub data: Option<OrderItemContainer>,
    pub error: Option<String>,
}
