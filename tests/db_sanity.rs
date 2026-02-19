mod common;

use diesel::prelude::*;
use proj_xs::db::DbConnection;

#[test]
fn db_migrations_run_and_empty_state() {
    let pool = common::setup_pool();

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

#[test]
fn pg_trgm_extension_present() {
    let pool = common::setup_pool();
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let ext_count: i64 = diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "SELECT COUNT(*) FROM pg_extension WHERE extname = 'pg_trgm'",
    )
    .get_result(conn.connection())
    .expect("querying pg_extension should succeed");

    assert_eq!(ext_count, 1, "pg_trgm extension should be installed");
}
