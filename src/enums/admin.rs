use crate::models::admin::{CanteenDetails, CanteenLoginSuccess, MenuItem, UpdateMenuItem};
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
    pub data: Vec<MenuItemWithPic>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MenuItemWithPic {
    pub item_id: i32,
    pub canteen_id: i32,
    pub name: String,
    pub is_veg: bool,
    pub price: i32,
    pub stock: i32,
    pub is_available: bool,
    pub description: Option<String>,
    pub pic_link: Option<String>,
    pub pic_etag: Option<String>,
}

impl From<&MenuItem> for MenuItemWithPic {
    fn from(item: &MenuItem) -> Self {
        MenuItemWithPic {
            item_id: item.item_id,
            canteen_id: item.canteen_id,
            name: item.name.clone(),
            is_veg: item.is_veg,
            price: item.price,
            stock: item.stock,
            is_available: item.is_available,
            description: item.description.clone(),
            pic_link: None,
            pic_etag: item.pic_etag.clone(),
        }
    }
}

impl Default for MenuItemWithPic {
    fn default() -> MenuItemWithPic {
        MenuItemWithPic {
            item_id: -1,
            canteen_id: -1,
            name: "".to_string(),
            is_veg: false,
            price: 0,
            stock: -1,
            is_available: false,
            description: None,
            pic_link: None,
            pic_etag: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CanteenDetailsWithPic {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
    pub pic_link: Option<String>,
    pub pic_etag: Option<String>,
}

impl From<&CanteenDetails> for CanteenDetailsWithPic {
    fn from(item: &CanteenDetails) -> Self {
        CanteenDetailsWithPic {
            canteen_id: item.canteen_id,
            canteen_name: item.canteen_name.clone(),
            location: item.location.clone(),
            pic_link: None,
            pic_etag: item.pic_etag.clone(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct ItemResponse {
    pub status: String,
    pub data: MenuItemWithPic,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct UploadMenuItemPicPresignedResponse {
    pub status: String,
    pub presigned_url: Option<String>,
    pub error: Option<String>,
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
pub struct UploadCanteenPicPresignedResponse {
    pub status: String,
    pub presigned_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AllCanteenResponse {
    pub status: String,
    pub data: Vec<CanteenDetailsWithPic>,
    pub error: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ItemUploadResponse {
    pub status: String,
    pub url: String,
    pub item_id: i32,
    pub error: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub status: String,
    pub error: Option<String>,
    pub data: Option<CanteenLoginSuccess>,
}
