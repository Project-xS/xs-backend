use chrono::{DateTime, Utc};
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Queryable, PartialEq, Selectable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::past_orders)]
#[diesel(primary_key(order_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PastOrder {
    pub order_id: i32,
    pub user_id: i32,
    pub items: Vec<Option<i32>>,
    pub order_status: bool,
    pub ordered_at: DateTime<Utc>,
    pub price: i32,
}

#[derive(Serialize, Debug)]
pub struct PastOrderItem {
    pub order_id: i32,
    pub canteen_id: i32,
    pub order_status: bool,
    pub ordered_at: DateTime<Utc>,
    pub total_price: i32,
    pub item_id: i32,
    pub name: String,
    pub quantity: i16,
    pub is_veg: bool,
    pub has_pic: bool,
    pub pic_etag: Option<String>,
    pub description: Option<String>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::past_orders)]
pub struct NewPastOrder {
    pub order_id: i32,
    pub user_id: i32,
    pub items: Vec<i32>,
    pub order_status: bool,
    pub ordered_at: DateTime<Utc>,
    pub price: i32,
}

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(primary_key(user_id))]
pub struct User {
    pub user_id: i32,
    pub rfid: String,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = crate::db::schema::users)]
pub struct NewUser {
    pub rfid: String,
    pub name: String,
    pub email: String,
}
