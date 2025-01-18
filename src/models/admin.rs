use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::canteens)]
#[diesel(primary_key(canteen_id))]
pub struct Canteen {
    pub canteen_id: i32,
    pub canteen_name: String,
    pub location: String,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
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

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
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

#[derive(Insertable, Debug, Serialize, Deserialize)]
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
