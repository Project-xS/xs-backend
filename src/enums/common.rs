use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ActiveItemCount {
    pub item_id: i32,
    pub item_name: String,
    pub num_ordered: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ActiveItemCountResponse {
    pub status: String,
    pub data: Vec<ActiveItemCount>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ItemContainer {
    pub canteen_name: String,
    pub name: String,
    pub quantity: i16,
    pub is_veg: bool,
    pub pic_link: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OrderItemContainer {
    pub order_id: i32,
    pub items: Vec<ItemContainer>
}

#[derive(Serialize, ToSchema)]
pub struct OrderItemsResponse {
    pub status: String,
    pub data: Vec<OrderItemContainer>,
    pub error: Option<String>,
}
