use crate::enums::common::ItemContainer;
use crate::models::user::PastOrderItem;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use with_pic_macro::{with_pic, WithPic};

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

#[with_pic(PastOrderItem)]
#[derive(Serialize, Debug, WithPic)]
pub struct PastOrderItemWithPic {
    pub order_id: i32,
    pub canteen_id: i32,
    pub order_status: bool,
    pub ordered_at: DateTime<Utc>,
    pub total_price: i32,
    pub item_id: i32,
    pub name: String,
    pub quantity: i16,
    pub is_veg: bool,
    pub pic_link: Option<String>,
    pub pic_etag: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, ToSchema, Debug)]
pub struct PastOrderItemContainer {
    pub order_id: i32,
    pub total_price: i32,
    pub order_status: bool,
    pub ordered_at: i64,
    pub items: Vec<ItemContainer>,
}

#[derive(Serialize, ToSchema)]
pub struct PastOrdersItemResponse {
    pub status: String,
    pub data: Vec<PastOrderItemContainer>,
    pub error: Option<String>,
}
