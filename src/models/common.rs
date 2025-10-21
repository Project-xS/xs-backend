use crate::db::schema::sql_types::TimeBand;
use chrono::{DateTime, Utc};
use diesel::deserialize::FromSql;
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::{
    serialize, AsExpression, Associations, FromSqlRow, Identifiable, Insertable, Queryable,
    Selectable,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use utoipa::ToSchema;

diesel::allow_columns_to_appear_in_same_group_by_clause!(
    crate::db::schema::menu_items::name,
    crate::db::schema::active_order_items::item_id,
    crate::db::schema::active_orders::deliver_at,
);

#[derive(
    Debug, PartialEq, FromSqlRow, AsExpression, Eq, Serialize, Deserialize, ToSchema, Hash, Clone,
)]
#[diesel(sql_type = TimeBand)]
pub enum TimeBandEnum {
    ElevenAM,
    TwevlvePM,
}

impl ToSql<TimeBand, diesel::pg::Pg> for TimeBandEnum {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
        match *self {
            TimeBandEnum::ElevenAM => out.write_all(b"ElevenAM")?,
            TimeBandEnum::TwevlvePM => out.write_all(b"TwelvePM")?,
        }
        Ok(IsNull::No)
    }
}

impl FromSql<TimeBand, diesel::pg::Pg> for TimeBandEnum {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> diesel::deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"ElevenAM" => Ok(TimeBandEnum::ElevenAM),
            b"TwelvePM" => Ok(TimeBandEnum::TwevlvePM),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl TimeBandEnum {
    pub fn human_readable(&self) -> &str {
        match self {
            Self::ElevenAM => "11:00am - 12:00pm",
            Self::TwevlvePM => "12:00pm - 01:00pm",
        }
    }
    pub fn get_enum_from_str(text: Option<&str>) -> Option<TimeBandEnum> {
        match text {
            Some("11:00am - 12:00pm") => Some(TimeBandEnum::ElevenAM),
            Some("12:00pm - 01:00pm") => Some(TimeBandEnum::TwevlvePM),
            _ => None,
        }
    }
}

#[allow(dead_code)]
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
    pub canteen_id: i32,
    pub price: i32,
    pub deliver_at: Option<TimeBandEnum>,
    pub ordered_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::active_orders)]
pub struct NewActiveOrder {
    pub user_id: i32,
    pub canteen_id: i32,
    pub total_price: i32,
    pub deliver_at: Option<TimeBandEnum>,
}

#[derive(Queryable, Serialize, ToSchema, Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OrderItems {
    pub order_id: i32,
    pub canteen_id: i32,
    pub item_id: i32,
    pub total_price: i32,
    pub deliver_at: Option<TimeBandEnum>,
    pub name: String,
    pub quantity: i16,
    pub is_veg: bool,
    pub has_pic: bool,
    pub pic_etag: Option<String>,
    pub description: Option<String>,
}
