use diesel::{Queryable, Selectable, Identifiable, Insertable};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

#[derive(Queryable, Selectable, Serialize, Deserialize, ToSchema, PartialEq, Debug)]
#[diesel(table_name = crate::db::schema::menu_items)]
pub struct OrderItems {
    pub item_id: i32,
    pub canteen_id: i32,
    pub name: String,
    // pub quantity: i32,
    pub is_veg: bool,
    pub pic_link: Option<String>,
    pub description: Option<String>,
}

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::active_orders)]
#[diesel(primary_key(order_id))]
pub struct ActiveOrder {
    pub order_id: String,
    pub user_id: i32,
    pub items: Vec<i32>,
    pub ordered_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::active_orders)]
pub struct NewActiveOrder {
    pub user_id: i32
}
