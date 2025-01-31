// @generated automatically by Diesel CLI.

diesel::table! {
    active_orders (order_id) {
        order_id -> Int4,
        user_id -> Int4,
        items -> Array<Nullable<Int4>>,
        ordered_at -> Timestamptz,
    }
}

diesel::table! {
    canteens (canteen_id) {
        canteen_id -> Int4,
        canteen_name -> Varchar,
        location -> Varchar,
    }
}

diesel::table! {
    menu_items (item_id) {
        item_id -> Int4,
        canteen_id -> Int4,
        name -> Varchar,
        is_veg -> Bool,
        price -> Float8,
        stock -> Int4,
        is_available -> Bool,
        list -> Bool,
        pic_link -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
    }
}

diesel::table! {
    past_orders (order_id) {
        order_id -> Varchar,
        user_id -> Int4,
        items -> Array<Nullable<Int4>>,
        order_status -> Bool,
        ordered_at -> Timestamptz,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Int4,
        rfid -> Varchar,
        name -> Varchar,
        email -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(active_orders -> users (user_id));
diesel::joinable!(menu_items -> canteens (canteen_id));
diesel::joinable!(past_orders -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    active_orders,
    canteens,
    menu_items,
    past_orders,
    users,
);
