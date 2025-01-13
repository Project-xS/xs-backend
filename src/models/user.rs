use chrono::{DateTime, Utc};
use diesel::{Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::past_orders)]
#[diesel(primary_key(order_id))]
pub struct PastOrder {
    pub order_id: String,
    pub user_id: i32,
    pub items: Vec<i32>,
    pub order_status: bool,
    pub ordered_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::past_orders)]
pub struct NewPastOrder {
    pub order_id: String,
    pub user_id: i32,
    pub items: Vec<i32>,
    pub order_status: bool,
    pub ordered_at: DateTime<Utc>,
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

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::users)]
pub struct NewUser {
    pub rfid: String,
    pub name: String,
    pub email: String,
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
    pub order_id: String,
    pub user_id: i32,
    pub items: Vec<i32>,
}
