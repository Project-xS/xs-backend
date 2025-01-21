use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
#[diesel(primary_key(canteen_id))]
pub struct Canteen {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
}

#[derive(Insertable, Debug, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::canteens)]
pub struct NewCanteen {
    pub canteen_name: String,
    pub location: String,
}

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::item_count)]
#[diesel(primary_key(item_id))]
pub struct ItemCount {
    pub item_id: i32,
    pub num_ordered: i32,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::item_count)]
pub struct NewItemCount {
    pub item_id: i32,
    pub num_ordered: i32,
}

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::menu_items)]
#[diesel(primary_key(item_id))]
pub struct MenuItem {
    pub item_id: i32,
    pub canteen_id: i32,
    pub name: String,
    pub is_veg: bool,
    pub price: f64,
    pub stock: i32,
    pub is_available: bool,
    pub list: bool,
    pub pic_link: Option<String>,
    pub description: Option<String>,
}

#[derive(Insertable, Debug, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::menu_items)]
pub struct NewMenuItem {
    pub canteen_id: i32,
    pub name: String,
    pub is_veg: bool,
    pub price: f64,
    pub stock: i32,
    pub is_available: bool,
    pub list: bool,
    pub pic_link: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = crate::db::schema::menu_items)]
pub struct UpdateMenuItem {
    pub name: Option<String>,
    pub is_veg: Option<bool>,
    pub price: Option<f64>,
    pub stock: Option<i32>,
    pub is_available: Option<bool>,
    pub list: Option<bool>,
    pub pic_link: Option<String>,
    pub description: Option<String>,
}
