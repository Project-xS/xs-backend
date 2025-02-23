use crate::models::admin::{Canteen, MenuItem, UpdateMenuItem};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct GeneralMenuResponse {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateMenuItemResponse {
    pub status: String,
    pub item_id: i32,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AllItemsResponse {
    pub status: String,
    pub data: Vec<MenuItem>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ItemResponse {
    pub status: String,
    pub data: MenuItem,
    pub error: Option<String>,
}

impl Default for MenuItem {
    fn default() -> MenuItem {
        MenuItem {
            item_id: -1,
            canteen_id: -1,
            name: "".to_string(),
            is_veg: false,
            price: -1.0,
            stock: -1,
            is_available: false,
            pic_link: None,
            description: None,
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateItemRequest {
    pub item_id: i32,
    pub update: UpdateMenuItem,
}

// ---------- CANTEEN ---------- //

#[derive(Serialize, ToSchema)]
pub struct NewCanteenResponse {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AllCanteenResponse {
    pub status: String,
    pub data: Vec<Canteen>,
    pub error: Option<String>,
}
