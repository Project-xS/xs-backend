use crate::db::{establish_connection_pool, run_db_migrations, DbConnection, RepositoryError};
use crate::models::admin::{NewCanteen, NewMenuItem};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use std::sync::Once;

// Fixture strategy:
// - Build users/canteens/menu items via helpers below.
// - Keep has_pic = false to avoid S3 presign calls in tests.
const TEST_S3_ENDPOINT: &str = "http://localhost:9000";
const TEST_S3_REGION: &str = "us-east-1";
const TEST_S3_ACCESS_KEY: &str = "test-access-key";
const TEST_S3_SECRET_KEY: &str = "test-secret-key";
const TEST_S3_BUCKET: &str = "test-bucket";
const TEST_DEV_BYPASS_TOKEN: &str = "test-bypass-token";
const TEST_FIREBASE_PROJECT_ID: &str = "test-project";
const TEST_ADMIN_JWT_SECRET: &str = "test-admin-secret";
const TEST_QR_HASH_SECRET: &str = "test-qr-secret";
static TEST_THREADS_GUARD: Once = Once::new();

fn ensure_single_threaded_tests() {
    TEST_THREADS_GUARD.call_once(|| {
        let threads = test_threads_from_args().or_else(|| std::env::var("RUST_TEST_THREADS").ok());
        if threads.as_deref() != Some("1") {
            panic!(
                "Tests must run with --test-threads=1 or RUST_TEST_THREADS=1 because init_test_env mutates environment variables."
            );
        }
    });
}

fn test_threads_from_args() -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == "--test-threads" {
            return args.next();
        }
        if let Some(value) = arg.strip_prefix("--test-threads=") {
            return Some(value.to_string());
        }
    }
    None
}

fn set_env_if_unset(key: &str, value: &str) {
    if std::env::var_os(key).is_none() {
        std::env::set_var(key, value);
    }
}

pub fn init_test_env() {
    ensure_single_threaded_tests();
    set_env_if_unset("S3_ENDPOINT", TEST_S3_ENDPOINT);
    set_env_if_unset("S3_REGION", TEST_S3_REGION);
    set_env_if_unset("S3_ACCESS_KEY_ID", TEST_S3_ACCESS_KEY);
    set_env_if_unset("S3_SECRET_KEY", TEST_S3_SECRET_KEY);
    set_env_if_unset("S3_BUCKET_NAME", TEST_S3_BUCKET);
    set_env_if_unset("AWS_EC2_METADATA_DISABLED", "true");
    set_env_if_unset("DEV_BYPASS_TOKEN", TEST_DEV_BYPASS_TOKEN);
    set_env_if_unset("FIREBASE_PROJECT_ID", TEST_FIREBASE_PROJECT_ID);
    set_env_if_unset("ADMIN_JWT_SECRET", TEST_ADMIN_JWT_SECRET);
    set_env_if_unset("DELIVER_QR_HASH_SECRET", TEST_QR_HASH_SECRET);
}

pub fn build_test_pool(database_url: &str) -> Pool<ConnectionManager<PgConnection>> {
    let pool = establish_connection_pool(database_url);
    run_db_migrations(pool.clone()).expect("Unable to run migrations");
    pool
}

pub fn reset_db(pool: &Pool<ConnectionManager<PgConnection>>) -> Result<(), RepositoryError> {
    let mut conn = DbConnection::new(pool)?;
    diesel::sql_query(
        "TRUNCATE TABLE active_order_items, active_orders, held_order_items, held_orders, \
         menu_items, past_orders, users, canteens RESTART IDENTITY CASCADE",
    )
    .execute(conn.connection())
    .map_err(RepositoryError::DatabaseError)?;
    Ok(())
}

pub struct TestFixtures {
    pub user_id: i32,
    pub canteen_id: i32,
    pub menu_item_ids: Vec<i32>,
}

pub fn seed_basic_fixtures(
    pool: &Pool<ConnectionManager<PgConnection>>,
) -> Result<TestFixtures, RepositoryError> {
    let mut conn = DbConnection::new(pool)?;

    let user_id = insert_user(
        conn.connection(),
        "test-user-1",
        "user1@example.com",
        "User One",
        Some("rfid-1"),
    )?;
    let canteen_id = insert_canteen(conn.connection(), "Test Canteen", "Block A")?;
    let veg_item_id = seed_menu_item(
        conn.connection(),
        canteen_id,
        "Veg Sandwich",
        120,
        10,
        true,
        true,
        Some("Simple veg sandwich"),
    )?;
    let non_veg_item_id = seed_menu_item(
        conn.connection(),
        canteen_id,
        "Chicken Wrap",
        180,
        5,
        true,
        false,
        Some("Spicy chicken wrap"),
    )?;

    Ok(TestFixtures {
        user_id,
        canteen_id,
        menu_item_ids: vec![veg_item_id, non_veg_item_id],
    })
}

pub fn insert_user(
    conn: &mut PgConnection,
    firebase_uid_val: &str,
    email_val: &str,
    name_val: &str,
    rfid_val: Option<&str>,
) -> Result<i32, RepositoryError> {
    use crate::db::schema::users::dsl::*;

    let rfid_value = rfid_val.map(|value| value.to_string());
    diesel::insert_into(users)
        .values((
            firebase_uid.eq(firebase_uid_val),
            email.eq(email_val),
            name.eq(name_val),
            rfid.eq(rfid_value),
        ))
        .returning(user_id)
        .get_result(conn)
        .map_err(RepositoryError::DatabaseError)
}

pub fn insert_canteen(
    conn: &mut PgConnection,
    canteen_name_val: &str,
    location_val: &str,
) -> Result<i32, RepositoryError> {
    use crate::db::schema::canteens::dsl::*;

    let new_canteen = NewCanteen {
        canteen_name: canteen_name_val.to_string(),
        location: location_val.to_string(),
        has_pic: false,
        opening_time: None,
        closing_time: None,
    };

    diesel::insert_into(canteens)
        .values(&new_canteen)
        .returning(canteen_id)
        .get_result(conn)
        .map_err(RepositoryError::DatabaseError)
}

#[allow(clippy::too_many_arguments)]
pub fn seed_menu_item(
    conn: &mut PgConnection,
    canteen_id_val: i32,
    name_val: &str,
    price_val: i32,
    stock_val: i32,
    is_available_val: bool,
    is_veg_val: bool,
    description_val: Option<&str>,
) -> Result<i32, RepositoryError> {
    use crate::db::schema::menu_items::dsl::*;

    let new_item = NewMenuItem {
        canteen_id: canteen_id_val,
        name: name_val.to_string(),
        is_veg: is_veg_val,
        price: price_val,
        stock: stock_val,
        is_available: is_available_val,
        description: description_val.map(|val| val.to_string()),
        has_pic: false,
    };

    diesel::insert_into(menu_items)
        .values(&new_item)
        .returning(item_id)
        .get_result(conn)
        .map_err(RepositoryError::DatabaseError)
}
