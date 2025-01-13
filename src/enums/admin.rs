use serde::{Deserialize, Serialize};
use crate::models::admin::MenuItem;

#[derive(Serialize)]
pub struct NewItemResponse {
    pub status: String,
    pub error: Option<String>
}

#[derive(Deserialize)]
pub struct ItemIdRequest {
    pub id: i32
}

#[derive(Serialize)]
pub struct AllItemsResponse {
    pub status: String,
    pub data: Vec<MenuItem>,
    pub error: Option<String>
}

#[derive(Serialize)]
pub struct ItemResponse {
    pub status: String,
    pub data: MenuItem,
    pub error: Option<String>
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
            list: false,
            pic_link: None,
            description: None,
        }
    }
}

#[derive(Deserialize)]
pub struct ReduceStockRequest {
    pub id: i32,
    pub amount: i32
}
