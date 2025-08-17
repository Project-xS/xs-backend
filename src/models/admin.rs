use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
#[diesel(primary_key(canteen_id))]
pub struct Canteen {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
    pub username: String,
    pub password: String,
    pub pic_link: bool,
}

#[derive(Queryable, Debug, Identifiable, Selectable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
#[diesel(primary_key(canteen_id))]
pub struct CanteenDetails {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
    pub pic_link: bool,
}

#[derive(Insertable, Debug, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct NewCanteen {
    pub canteen_name: String,
    pub location: String,
    pub pic_link: bool,
}

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize, ToSchema, Selectable)]
#[diesel(table_name = crate::db::schema::menu_items)]
#[diesel(primary_key(item_id))]
pub struct MenuItem {
    pub item_id: i32,
    pub canteen_id: i32,
    pub name: String,
    pub is_veg: bool,
    pub price: i32,
    pub stock: i32,
    pub is_available: bool,
    pub description: Option<String>,
    pub pic_link: bool,
}

#[derive(Insertable, Debug, Serialize, Deserialize, ToSchema, Selectable)]
#[diesel(table_name = crate::db::schema::menu_items)]
pub struct NewMenuItem {
    pub canteen_id: i32,
    pub name: String,
    pub is_veg: bool,
    pub price: i32,
    pub stock: i32,
    pub is_available: bool,
    pub description: Option<String>,
    pub pic_link: bool,
}

#[derive(Debug, Selectable, Queryable)]
#[diesel(table_name = crate::db::schema::menu_items)]
#[diesel(primary_key(item_id))]
pub struct MenuItemCheck {
    pub item_id: i32,
    pub canteen_id: i32,
    pub name: String,
    pub stock: i32,
    pub price: i32,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = crate::db::schema::menu_items)]
pub struct UpdateMenuItem {
    pub name: Option<String>,
    pub is_veg: Option<bool>,
    pub price: Option<i32>,
    pub stock: Option<i32>,
    pub is_available: Option<bool>,
    pub description: Option<String>,
    pub pic_link: bool,
}

#[derive(Debug, Selectable, Queryable, Serialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct CanteenLoginSuccess {
    pub canteen_id: i32,
    pub canteen_name: String,
}
