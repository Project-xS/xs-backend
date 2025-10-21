// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "time_band"))]
    pub struct TimeBand;
}

diesel::table! {
    active_order_items (order_id, item_id) {
        order_id -> Int4,
        item_id -> Int4,
        quantity -> Int2,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TimeBand;

    active_orders (order_id) {
        order_id -> Int4,
        user_id -> Int4,
        ordered_at -> Timestamptz,
        total_price -> Int4,
        canteen_id -> Int4,
        deliver_at -> Nullable<TimeBand>,
    }
}

diesel::table! {
    canteens (canteen_id) {
        canteen_id -> Int4,
        canteen_name -> Varchar,
        location -> Varchar,
        username -> Varchar,
        password -> Varchar,
        has_pic -> Bool,
        pic_etag -> Nullable<Varchar>,
    }
}

diesel::table! {
    menu_items (item_id) {
        item_id -> Int4,
        canteen_id -> Int4,
        name -> Varchar,
        is_veg -> Bool,
        price -> Int4,
        stock -> Int4,
        is_available -> Bool,
        description -> Nullable<Varchar>,
        has_pic -> Bool,
        pic_etag -> Nullable<Varchar>,
    }
}

diesel::table! {
    past_orders (order_id) {
        order_id -> Int4,
        user_id -> Int4,
        items -> Array<Nullable<Int4>>,
        order_status -> Bool,
        ordered_at -> Timestamptz,
        price -> Int4,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Int4,
        rfid -> Nullable<Varchar>,
        name -> Varchar,
        email -> Varchar,
        firebase_uid -> Text,
        auth_provider -> Text,
        email_verified -> Bool,
        display_name -> Nullable<Text>,
        photo_url -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(active_order_items -> active_orders (order_id));
diesel::joinable!(active_order_items -> menu_items (item_id));
diesel::joinable!(active_orders -> canteens (canteen_id));
diesel::joinable!(active_orders -> users (user_id));
diesel::joinable!(menu_items -> canteens (canteen_id));
diesel::joinable!(past_orders -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    active_order_items,
    active_orders,
    canteens,
    menu_items,
    past_orders,
    users,
);
