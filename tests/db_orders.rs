mod common;

use diesel::prelude::*;
use diesel::PgConnection;
use proj_xs::db::{DbConnection, OrderOperations, RepositoryError};
use proj_xs::models::common::TimeBandEnum;
use proj_xs::test_utils::{insert_canteen, seed_menu_item};

fn active_orders_count(conn: &mut PgConnection) -> i64 {
    proj_xs::db::schema::active_orders::table
        .count()
        .get_result(conn)
        .expect("count active_orders")
}

fn active_order_items_count(conn: &mut PgConnection) -> i64 {
    proj_xs::db::schema::active_order_items::table
        .count()
        .get_result(conn)
        .expect("count active_order_items")
}

fn past_orders_count(conn: &mut PgConnection) -> i64 {
    proj_xs::db::schema::past_orders::table
        .count()
        .get_result(conn)
        .expect("count past_orders")
}

fn menu_item_state(conn: &mut PgConnection, item_id_val: i32) -> (i32, bool) {
    use proj_xs::db::schema::menu_items::dsl::*;
    menu_items
        .filter(item_id.eq(item_id_val))
        .select((stock, is_available))
        .first::<(i32, bool)>(conn)
        .expect("menu item state")
}

#[actix_rt::test]
async fn create_order_success_decrements_stock_and_creates_rows() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let veg_item = fixtures.menu_item_ids[0];
    let non_veg_item = fixtures.menu_item_ids[1];

    use proj_xs::db::schema::menu_items::dsl as menu_items_dsl;
    diesel::update(menu_items_dsl::menu_items.filter(menu_items_dsl::item_id.eq(veg_item)))
        .set((
            menu_items_dsl::stock.eq(2),
            menu_items_dsl::is_available.eq(true),
        ))
        .execute(conn.connection())
        .expect("set veg stock");
    diesel::update(menu_items_dsl::menu_items.filter(menu_items_dsl::item_id.eq(non_veg_item)))
        .set((
            menu_items_dsl::stock.eq(5),
            menu_items_dsl::is_available.eq(true),
        ))
        .execute(conn.connection())
        .expect("set non-veg stock");

    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(
            fixtures.user_id,
            vec![veg_item, veg_item, non_veg_item],
            Some("11:00am - 12:00pm".to_string()),
        )
        .expect("create order");

    use proj_xs::db::schema::active_orders::dsl as active_orders_dsl;
    let (order_id_val, total_price_val, deliver_at_val) = active_orders_dsl::active_orders
        .select((
            active_orders_dsl::order_id,
            active_orders_dsl::total_price,
            active_orders_dsl::deliver_at,
        ))
        .first::<(i32, i32, Option<TimeBandEnum>)>(conn.connection())
        .expect("active order");
    assert_eq!(total_price_val, 2 * 120 + 180);
    assert_eq!(deliver_at_val, Some(TimeBandEnum::ElevenAM));

    use proj_xs::db::schema::active_order_items::dsl as active_order_items_dsl;
    let items = active_order_items_dsl::active_order_items
        .filter(active_order_items_dsl::order_id.eq(order_id_val))
        .select((
            active_order_items_dsl::item_id,
            active_order_items_dsl::quantity,
            active_order_items_dsl::price,
        ))
        .load::<(i32, i16, i32)>(conn.connection())
        .expect("order items");

    assert_eq!(items.len(), 2);
    let mut found_veg = false;
    let mut found_non_veg = false;
    for (item, qty, price) in items {
        if item == veg_item {
            found_veg = true;
            assert_eq!(qty, 2);
            assert_eq!(price, 120);
        }
        if item == non_veg_item {
            found_non_veg = true;
            assert_eq!(qty, 1);
            assert_eq!(price, 180);
        }
    }
    assert!(found_veg);
    assert!(found_non_veg);

    let (veg_stock, veg_available) = menu_item_state(conn.connection(), veg_item);
    let (non_veg_stock, non_veg_available) = menu_item_state(conn.connection(), non_veg_item);
    assert_eq!(veg_stock, 0);
    assert!(!veg_available);
    assert_eq!(non_veg_stock, 4);
    assert!(non_veg_available);
}

#[actix_rt::test]
async fn create_order_fails_on_cross_canteen_items() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let other_canteen =
        insert_canteen(conn.connection(), "Other Canteen", "Block B").expect("insert canteen");
    let other_item = seed_menu_item(
        conn.connection(),
        other_canteen,
        "Other Item",
        99,
        10,
        true,
        true,
        None,
    )
    .expect("seed menu item");

    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(fixtures.menu_item_ids[0])))
        .set((stock.eq(5), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");
    diesel::update(menu_items.filter(item_id.eq(other_item)))
        .set((stock.eq(5), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock other");

    let order_ops = OrderOperations::new(pool.clone()).await;
    let err = order_ops
        .create_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0], other_item],
            None,
        )
        .expect_err("cross-canteen should fail");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    assert_eq!(active_orders_count(conn.connection()), 0);
    assert_eq!(active_order_items_count(conn.connection()), 0);

    let (stock_a, _) = menu_item_state(conn.connection(), fixtures.menu_item_ids[0]);
    let (stock_b, _) = menu_item_state(conn.connection(), other_item);
    assert_eq!(stock_a, 5);
    assert_eq!(stock_b, 5);
}

#[actix_rt::test]
async fn create_order_fails_on_out_of_stock() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(0), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    let order_ops = OrderOperations::new(pool.clone()).await;
    let err = order_ops
        .create_order(fixtures.user_id, vec![item_id_val], None)
        .expect_err("out of stock");
    assert!(matches!(err, RepositoryError::NotAvailable(..)));

    assert_eq!(active_orders_count(conn.connection()), 0);
    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, 0);
    assert!(available_val);
}

#[actix_rt::test]
async fn create_order_fails_on_unavailable_item() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(10), is_available.eq(false)))
        .execute(conn.connection())
        .expect("set availability");

    let order_ops = OrderOperations::new(pool.clone()).await;
    let err = order_ops
        .create_order(fixtures.user_id, vec![item_id_val], None)
        .expect_err("unavailable");
    assert!(matches!(err, RepositoryError::NotAvailable(..)));

    assert_eq!(active_orders_count(conn.connection()), 0);
    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, 10);
    assert!(!available_val);
}

#[actix_rt::test]
async fn create_order_unlimited_stock_preserves_negative_one() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(-1), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![item_id_val], None)
        .expect("create order");

    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, -1);
    assert!(available_val);
}

#[actix_rt::test]
async fn create_order_fails_on_empty_or_missing_items() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let order_ops = OrderOperations::new(pool.clone()).await;

    let err = order_ops
        .create_order(fixtures.user_id, vec![], None)
        .expect_err("empty order");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    let err = order_ops
        .create_order(fixtures.user_id, vec![9999], None)
        .expect_err("missing item");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    let mut conn = DbConnection::new(&pool).expect("db connection");
    assert_eq!(active_orders_count(conn.connection()), 0);
    assert_eq!(active_order_items_count(conn.connection()), 0);
}

#[actix_rt::test]
async fn create_order_invalid_deliver_at_maps_to_none() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0]],
            Some("invalid".to_string()),
        )
        .expect("create order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let deliver_at_val = active_orders
        .select(deliver_at)
        .first::<Option<TimeBandEnum>>(conn.connection())
        .expect("deliver_at");
    assert!(deliver_at_val.is_none());
}

#[actix_rt::test]
async fn get_orders_by_userid_groups_items() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0], fixtures.menu_item_ids[1]],
            None,
        )
        .expect("create order");

    let orders = order_ops
        .get_orders_by_userid(&fixtures.user_id)
        .await
        .expect("get orders");
    assert_eq!(orders.len(), 1);
    let order = &orders[0];
    assert_eq!(order.items.len(), 2);
    assert_eq!(order.deliver_at, "Instant");
    assert_eq!(order.total_price, 120 + 180);
}

#[actix_rt::test]
async fn get_orders_by_rfid_returns_orders() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0]],
            Some("12:00pm - 01:00pm".to_string()),
        )
        .expect("create order");

    let orders = order_ops
        .get_orders_by_rfid("rfid-1")
        .await
        .expect("get orders by rfid");
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].deliver_at, "12:00pm - 01:00pm");
}

#[actix_rt::test]
async fn get_all_orders_by_count_groups_by_time_band() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let order_ops = OrderOperations::new(pool.clone()).await;

    order_ops
        .create_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0]],
            Some("11:00am - 12:00pm".to_string()),
        )
        .expect("create order 1");
    order_ops
        .create_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0], fixtures.menu_item_ids[0]],
            Some("11:00am - 12:00pm".to_string()),
        )
        .expect("create order 2");
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[1]], None)
        .expect("create order 3");

    let grouped = order_ops
        .get_all_orders_by_count(fixtures.canteen_id)
        .expect("grouped orders");

    let eleven = grouped.get("11:00am - 12:00pm").expect("11am band");
    assert_eq!(eleven.len(), 1);
    assert_eq!(eleven[0].item_id, fixtures.menu_item_ids[0]);
    assert_eq!(eleven[0].num_ordered, 3);

    let instant = grouped.get("Instant").expect("instant band");
    assert_eq!(instant.len(), 1);
    assert_eq!(instant[0].item_id, fixtures.menu_item_ids[1]);
    assert_eq!(instant[0].num_ordered, 1);
}

#[actix_rt::test]
async fn order_actions_moves_to_past_orders() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0], fixtures.menu_item_ids[1]],
            None,
        )
        .expect("create order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl as active_orders_dsl;
    let order_id_val = active_orders_dsl::active_orders
        .select(active_orders_dsl::order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    order_ops
        .order_actions(&order_id_val, "delivered")
        .expect("deliver order");

    assert_eq!(active_orders_count(conn.connection()), 0);
    assert_eq!(active_order_items_count(conn.connection()), 0);
    assert_eq!(past_orders_count(conn.connection()), 1);

    use proj_xs::db::schema::past_orders::dsl as past_orders_dsl;
    let order_status_val = past_orders_dsl::past_orders
        .filter(past_orders_dsl::order_id.eq(order_id_val))
        .select(past_orders_dsl::order_status)
        .first::<bool>(conn.connection())
        .expect("past order status");
    assert!(order_status_val);
}

#[actix_rt::test]
async fn order_actions_handles_cancelled_and_missing() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    order_ops
        .order_actions(&order_id_val, "cancelled")
        .expect("cancel order");
    assert_eq!(active_orders_count(conn.connection()), 0);
    assert_eq!(past_orders_count(conn.connection()), 1);

    let err = order_ops
        .order_actions(&9999, "delivered")
        .expect_err("missing order");
    assert!(matches!(err, RepositoryError::NotFound(_)));
}
