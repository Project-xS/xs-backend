mod common;

use diesel::prelude::*;
use proj_xs::db::DbConnection;
use proj_xs::test_utils::{build_test_pool, init_test_env, reset_db};

#[test]
fn db_migrations_run_and_empty_state() {
    init_test_env();
    let db = common::setup_test_db();
    let pool = build_test_pool(&db.database_url);
    reset_db(&pool).expect("reset db");

    let mut conn = DbConnection::new(&pool).expect("db connection");

    let user_count: i64 = proj_xs::db::schema::users::table
        .count()
        .get_result(conn.connection())
        .expect("count users");
    let canteen_count: i64 = proj_xs::db::schema::canteens::table
        .count()
        .get_result(conn.connection())
        .expect("count canteens");
    let menu_item_count: i64 = proj_xs::db::schema::menu_items::table
        .count()
        .get_result(conn.connection())
        .expect("count menu items");

    assert_eq!(user_count, 0);
    assert_eq!(canteen_count, 0);
    assert_eq!(menu_item_count, 0);
}
