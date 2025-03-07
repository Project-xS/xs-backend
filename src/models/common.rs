use chrono::{DateTime, Utc};
use diesel::{Associations, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Queryable, Selectable, Serialize, Deserialize, ToSchema, Associations, Debug)]
#[diesel(table_name = crate::db::schema::active_order_items)]
#[diesel(belongs_to(ActiveOrder, foreign_key = order_id))]
pub struct ActiveOrderItems {
    pub order_id: i32,
    pub item_id: i32,
    pub quantity: i32,
}

#[derive(Queryable, Debug, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::active_orders)]
#[diesel(primary_key(order_id))]
pub struct ActiveOrder {
    pub order_id: String,
    pub user_id: i32,
    pub price: i32,
    pub ordered_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::active_orders)]
pub struct NewActiveOrder {
    pub user_id: i32,
    pub price: i32
}

#[derive(Queryable, Serialize, ToSchema)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OrderItems {
    pub order_id: i32,
    pub canteen_name: String,
    pub name: String,
    pub quantity: i16,
    pub is_veg: bool,
    pub pic_link: Option<String>,
    pub description: Option<String>,
}
