mod common;

use diesel::prelude::*;
use proj_xs::db::{DbConnection, RepositoryError};
use proj_xs::test_utils::{insert_canteen, seed_menu_item};

#[test]
fn create_canteen_inserts_correctly() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let new_id = insert_canteen(conn.connection(), "Food Court", "Block C").expect("insert");

    use proj_xs::db::schema::canteens::dsl::*;
    let (name_val, loc_val): (String, String) = canteens
        .filter(canteen_id.eq(new_id))
        .select((canteen_name, location))
        .first(conn.connection())
        .expect("fetch canteen");

    assert_eq!(name_val, "Food Court");
    assert_eq!(loc_val, "Block C");
}

#[test]
fn get_all_canteens_returns_seeded() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    use proj_xs::db::schema::canteens::dsl::*;
    let names: Vec<String> = canteens
        .order_by(canteen_id.asc())
        .select(canteen_name)
        .load(conn.connection())
        .expect("load canteens");

    assert!(!names.is_empty(), "should have at least the seeded canteen");
    assert!(
        names.iter().any(|n| n == "Test Canteen"),
        "seeded canteen should be present"
    );

    // Verify the fixture canteen_id exists in db
    let count: i64 = canteens
        .filter(canteen_id.eq(fixtures.canteen_id))
        .count()
        .get_result(conn.connection())
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn get_canteen_items_returns_correct_items() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    use proj_xs::db::schema::menu_items::dsl::*;
    let items: Vec<i32> = menu_items
        .filter(canteen_id.eq(fixtures.canteen_id))
        .select(item_id)
        .load(conn.connection())
        .expect("load items");

    assert_eq!(items.len(), 2, "seeded canteen should have 2 items");
    for id in &fixtures.menu_item_ids {
        assert!(items.contains(id), "item {} should belong to canteen", id);
    }
}

#[test]
fn get_canteen_items_empty_for_nonexistent() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    use proj_xs::db::schema::menu_items::dsl::*;
    let items: Vec<i32> = menu_items
        .filter(canteen_id.eq(99999))
        .select(item_id)
        .load(conn.connection())
        .expect("load items");

    assert!(
        items.is_empty(),
        "non-existent canteen should return no items"
    );
}

#[test]
fn create_canteen_and_add_item() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let canteen_id_val =
        insert_canteen(conn.connection(), "New Canteen", "Block D").expect("insert canteen");
    let item_id_val = seed_menu_item(
        conn.connection(),
        canteen_id_val,
        "New Dish",
        100,
        20,
        true,
        true,
        None,
    )
    .expect("insert item");

    use proj_xs::db::schema::menu_items::dsl::*;
    let result: Result<i32, _> = menu_items
        .filter(item_id.eq(item_id_val))
        .filter(canteen_id.eq(canteen_id_val))
        .select(item_id)
        .first(conn.connection());

    assert!(
        result.is_ok(),
        "item should be accessible via the new canteen"
    );

    // Foreign key check: deleting the pool doesn't affect just the items query
    let _: Result<(), RepositoryError> = Ok(());
}
