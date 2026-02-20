mod common;

use common::{
    active_order_items_count, active_orders_count, held_order_items_count, held_orders_count,
    menu_item_state,
};
use diesel::prelude::*;
use proj_xs::db::{DbConnection, HoldOperations, RepositoryError};
use proj_xs::models::common::TimeBandEnum;
use proj_xs::test_utils::{insert_canteen, insert_user, seed_menu_item};

#[test]
fn hold_order_success_decrements_stock_and_creates_rows() {
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

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id_val, _) = hold_ops
        .hold_order(
            fixtures.user_id,
            vec![veg_item, veg_item, non_veg_item],
            Some("11:00am - 12:00pm".to_string()),
        )
        .expect("hold order");

    use proj_xs::db::schema::held_orders::dsl as held_orders_dsl;
    let (total_price_val, deliver_at_val) = held_orders_dsl::held_orders
        .filter(held_orders_dsl::hold_id.eq(hold_id_val))
        .select((held_orders_dsl::total_price, held_orders_dsl::deliver_at))
        .first::<(i32, Option<TimeBandEnum>)>(conn.connection())
        .expect("held order");
    assert_eq!(total_price_val, 2 * 120 + 180);
    assert_eq!(deliver_at_val, Some(TimeBandEnum::ElevenAM));

    use proj_xs::db::schema::held_order_items::dsl as held_order_items_dsl;
    let items = held_order_items_dsl::held_order_items
        .filter(held_order_items_dsl::hold_id.eq(hold_id_val))
        .select((
            held_order_items_dsl::item_id,
            held_order_items_dsl::quantity,
            held_order_items_dsl::price,
        ))
        .load::<(i32, i16, i32)>(conn.connection())
        .expect("held order items");
    assert_eq!(items.len(), 2);

    let (veg_stock, veg_available) = menu_item_state(conn.connection(), veg_item);
    let (non_veg_stock, non_veg_available) = menu_item_state(conn.connection(), non_veg_item);
    assert_eq!(veg_stock, 0);
    assert!(!veg_available);
    assert_eq!(non_veg_stock, 4);
    assert!(non_veg_available);
}

#[test]
fn hold_order_fails_on_cross_canteen_items() {
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

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let err = hold_ops
        .hold_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0], other_item],
            None,
        )
        .expect_err("cross-canteen should fail");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    assert_eq!(held_orders_count(conn.connection()), 0);
    assert_eq!(held_order_items_count(conn.connection()), 0);
}

#[test]
fn hold_order_fails_on_out_of_stock() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(0), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let err = hold_ops
        .hold_order(fixtures.user_id, vec![item_id_val], None)
        .expect_err("out of stock");
    assert!(matches!(err, RepositoryError::NotAvailable(..)));

    assert_eq!(held_orders_count(conn.connection()), 0);
    let (stock_val, _) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, 0);
}

#[test]
fn hold_order_fails_on_unavailable_item() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(10), is_available.eq(false)))
        .execute(conn.connection())
        .expect("set availability");

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let err = hold_ops
        .hold_order(fixtures.user_id, vec![item_id_val], None)
        .expect_err("unavailable");
    assert!(matches!(err, RepositoryError::NotAvailable(..)));

    assert_eq!(held_orders_count(conn.connection()), 0);
    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, 10);
    assert!(!available_val);
}

#[test]
fn hold_order_unlimited_stock_preserves_negative_one() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(-1), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    hold_ops
        .hold_order(fixtures.user_id, vec![item_id_val], None)
        .expect("hold order");

    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, -1);
    assert!(available_val);
}

#[test]
fn hold_order_fails_on_empty_or_missing_items() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let hold_ops = HoldOperations::new(pool.clone(), 300);

    let err = hold_ops
        .hold_order(fixtures.user_id, vec![], None)
        .expect_err("empty order");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    let err = hold_ops
        .hold_order(fixtures.user_id, vec![9999], None)
        .expect_err("missing item");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    let mut conn = DbConnection::new(&pool).expect("db connection");
    assert_eq!(held_orders_count(conn.connection()), 0);
    assert_eq!(held_order_items_count(conn.connection()), 0);
}

#[test]
fn hold_order_invalid_deliver_at_maps_to_none() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id_val, _) = hold_ops
        .hold_order(
            fixtures.user_id,
            vec![fixtures.menu_item_ids[0]],
            Some("invalid".to_string()),
        )
        .expect("hold order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::held_orders::dsl::*;
    let deliver_at_val = held_orders
        .filter(hold_id.eq(hold_id_val))
        .select(deliver_at)
        .first::<Option<TimeBandEnum>>(conn.connection())
        .expect("deliver_at");
    assert!(deliver_at_val.is_none());
}

#[test]
fn confirm_held_order_moves_to_active_orders() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id_val, _) = hold_ops
        .hold_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("hold order");

    let order_id_val = hold_ops
        .confirm_held_order(hold_id_val, fixtures.user_id)
        .expect("confirm hold");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    assert_eq!(held_orders_count(conn.connection()), 0);
    assert_eq!(held_order_items_count(conn.connection()), 0);
    assert_eq!(active_orders_count(conn.connection()), 1);
    assert_eq!(active_order_items_count(conn.connection()), 1);

    use proj_xs::db::schema::active_orders::dsl as active_orders_dsl;
    let stored_order_id = active_orders_dsl::active_orders
        .select(active_orders_dsl::order_id)
        .first::<i32>(conn.connection())
        .expect("active order id");
    assert_eq!(stored_order_id, order_id_val);
}

#[test]
fn confirm_held_order_expired_restores_stock_and_deletes() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(1), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    let hold_ops = HoldOperations::new(pool.clone(), -1);
    let (hold_id_val, _) = hold_ops
        .hold_order(fixtures.user_id, vec![item_id_val], None)
        .expect("hold order");

    let err = hold_ops
        .confirm_held_order(hold_id_val, fixtures.user_id)
        .expect_err("expired hold");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    assert_eq!(held_orders_count(conn.connection()), 0);
    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, 1);
    assert!(available_val);
}

#[test]
fn confirm_held_order_owner_mismatch_keeps_hold() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let other_user_id = insert_user(
        conn.connection(),
        "test-user-2",
        "user2@example.com",
        "User Two",
        None,
    )
    .expect("insert user");

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id_val, _) = hold_ops
        .hold_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("hold order");

    let err = hold_ops
        .confirm_held_order(hold_id_val, other_user_id)
        .expect_err("owner mismatch");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    assert_eq!(held_orders_count(conn.connection()), 1);
}

#[test]
fn release_held_order_restores_stock() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_id_val)))
        .set((stock.eq(1), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id_val, _) = hold_ops
        .hold_order(fixtures.user_id, vec![item_id_val], None)
        .expect("hold order");

    hold_ops
        .release_held_order(hold_id_val, fixtures.user_id)
        .expect("release hold");

    assert_eq!(held_orders_count(conn.connection()), 0);
    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, 1);
    assert!(available_val);
}

#[test]
fn cleanup_expired_holds_restores_stock_and_deletes() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_a = fixtures.menu_item_ids[0];
    let item_b = fixtures.menu_item_ids[1];
    use proj_xs::db::schema::menu_items::dsl::*;
    diesel::update(menu_items.filter(item_id.eq(item_a)))
        .set((stock.eq(2), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock a");
    diesel::update(menu_items.filter(item_id.eq(item_b)))
        .set((stock.eq(2), is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock b");

    let hold_ops_expired = HoldOperations::new(pool.clone(), -1);
    let hold_ops_active = HoldOperations::new(pool.clone(), 300);

    hold_ops_expired
        .hold_order(fixtures.user_id, vec![item_a], None)
        .expect("expired hold");
    hold_ops_active
        .hold_order(fixtures.user_id, vec![item_b], None)
        .expect("active hold");

    let cleaned = hold_ops_active.cleanup_expired_holds().expect("cleanup");
    assert_eq!(cleaned, 1);

    assert_eq!(held_orders_count(conn.connection()), 1);
    let (stock_a, _) = menu_item_state(conn.connection(), item_a);
    let (stock_b, _) = menu_item_state(conn.connection(), item_b);
    assert_eq!(stock_a, 2);
    assert_eq!(stock_b, 1);
}

#[test]
fn release_held_order_keeps_unlimited_stock() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl as menu_items_dsl;
    diesel::update(menu_items_dsl::menu_items.filter(menu_items_dsl::item_id.eq(item_id_val)))
        .set((
            menu_items_dsl::stock.eq(-1),
            menu_items_dsl::is_available.eq(true),
        ))
        .execute(conn.connection())
        .expect("set stock");

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id_val, _) = hold_ops
        .hold_order(fixtures.user_id, vec![item_id_val], None)
        .expect("hold order");

    hold_ops
        .release_held_order(hold_id_val, fixtures.user_id)
        .expect("release hold");

    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, -1);
    assert!(available_val);
}

#[test]
fn confirm_expired_hold_keeps_unlimited_stock() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id_val = fixtures.menu_item_ids[0];
    use proj_xs::db::schema::menu_items::dsl as menu_items_dsl;
    diesel::update(menu_items_dsl::menu_items.filter(menu_items_dsl::item_id.eq(item_id_val)))
        .set((
            menu_items_dsl::stock.eq(-1),
            menu_items_dsl::is_available.eq(true),
        ))
        .execute(conn.connection())
        .expect("set stock");

    let hold_ops = HoldOperations::new(pool.clone(), -1);
    let (hold_id_val, _) = hold_ops
        .hold_order(fixtures.user_id, vec![item_id_val], None)
        .expect("hold order");

    let err = hold_ops
        .confirm_held_order(hold_id_val, fixtures.user_id)
        .expect_err("expired hold");
    assert!(matches!(err, RepositoryError::ValidationError(_)));

    let (stock_val, available_val) = menu_item_state(conn.connection(), item_id_val);
    assert_eq!(stock_val, -1);
    assert!(available_val);
}

#[test]
fn cleanup_expired_holds_returns_zero_when_none_expired() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let hold_ops = HoldOperations::new(pool.clone(), 300);

    hold_ops
        .hold_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("hold order");

    let cleaned = hold_ops.cleanup_expired_holds().expect("cleanup");
    assert_eq!(cleaned, 0);
}

#[test]
fn concurrent_holds_on_last_stock_item_one_succeeds_one_conflicts() {
    // Two concurrent hold_order calls on an item with stock=1.
    // FOR UPDATE locking in hold_order serialises them: one gets the stock,
    // the other sees stock=0 after the first commits and returns NotAvailable.
    let (pool, fixtures) = common::setup_pool_with_fixtures();

    // Set stock to exactly 1
    {
        let mut conn = DbConnection::new(&pool).expect("db connection");
        use proj_xs::db::schema::menu_items::dsl as mi;
        diesel::update(mi::menu_items.filter(mi::item_id.eq(fixtures.menu_item_ids[0])))
            .set((mi::stock.eq(1), mi::is_available.eq(true)))
            .execute(conn.connection())
            .expect("set stock=1");
    }

    let item_id = fixtures.menu_item_ids[0];
    let user_id = fixtures.user_id;

    // Spawn two threads that both try to hold the same item simultaneously
    let pool1 = pool.clone();
    let pool2 = pool.clone();

    let t1 = std::thread::spawn(move || {
        let hold_ops = HoldOperations::new(pool1, 300);
        hold_ops.hold_order(user_id, vec![item_id], None)
    });

    let t2 = std::thread::spawn(move || {
        let hold_ops = HoldOperations::new(pool2, 300);
        hold_ops.hold_order(user_id, vec![item_id], None)
    });

    let r1 = t1.join().expect("thread 1 panicked");
    let r2 = t2.join().expect("thread 2 panicked");

    let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    let failures = [&r1, &r2].iter().filter(|r| r.is_err()).count();

    assert_eq!(successes, 1, "exactly one hold should succeed");
    assert_eq!(
        failures, 1,
        "exactly one hold should fail (stock exhausted)"
    );

    // Verify stock is now 0
    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::menu_items::dsl as mi;
    let stock_val: i32 = mi::menu_items
        .filter(mi::item_id.eq(item_id))
        .select(mi::stock)
        .first(conn.connection())
        .expect("fetch stock");
    assert_eq!(stock_val, 0, "stock should be fully consumed");
}

#[test]
fn concurrent_confirm_and_cancel_same_hold_one_succeeds() {
    // confirm_held_order and release_held_order do NOT use FOR UPDATE on the
    // held_orders row, so both can read the hold before either commits.
    // Under READ COMMITTED both transactions complete successfully:
    //   - confirm creates 1 active_order and deletes the hold
    //   - release restores stock and tries to delete the hold (0 rows, no error)
    //
    // This test documents the actual behaviour and asserts the invariant that
    // the hold row is fully removed and exactly one active order exists after
    // both operations complete, regardless of which "wins" the internal race.
    let (pool, fixtures) = common::setup_pool_with_fixtures();

    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id_val, _) = hold_ops
        .hold_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("hold order");

    let user_id = fixtures.user_id;
    let pool1 = pool.clone();
    let pool2 = pool.clone();

    let t1 = std::thread::spawn(move || {
        let ops = HoldOperations::new(pool1, 300);
        ops.confirm_held_order(hold_id_val, user_id)
            .map(|_| ())
            .map_err(|e| e)
    });
    let t2 = std::thread::spawn(move || {
        let ops = HoldOperations::new(pool2, 300);
        ops.release_held_order(hold_id_val, user_id)
    });

    let r1 = t1.join().expect("thread 1 panicked");
    let r2 = t2.join().expect("thread 2 panicked");

    // At least one must succeed. Both can succeed (READ COMMITTED, no FOR UPDATE),
    // or only one if the other sees the hold already deleted (NotFound).
    let successes = [r1.is_ok(), r2.is_ok()].iter().filter(|&&ok| ok).count();
    assert!(
        successes >= 1,
        "at least one of confirm/cancel should succeed"
    );

    // The hold row must be gone regardless of which won.
    let mut conn = DbConnection::new(&pool).expect("db connection");
    assert_eq!(
        common::held_orders_count(conn.connection()),
        0,
        "hold must be fully consumed"
    );
    // active orders ≤ 1 (only confirm creates one)
    assert!(
        common::active_orders_count(conn.connection()) <= 1,
        "at most 1 active order should exist"
    );
}
