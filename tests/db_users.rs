mod common;

use proj_xs::db::{OrderOperations, RepositoryError, UserOperations};
use proj_xs::test_utils::insert_user;

#[actix_rt::test]
async fn upsert_firebase_user_insert_and_update() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let user_ops = UserOperations::new(pool.clone()).await;

    // Insert a new user
    let user = user_ops
        .upsert_firebase_user(
            "firebase-uid-test".to_string(),
            Some("test@example.com".to_string()),
            Some("Test User".to_string()),
            None,
            true,
        )
        .expect("upsert insert should succeed");

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, "Test User");

    // Upsert again with different email - should update
    let updated_user = user_ops
        .upsert_firebase_user(
            "firebase-uid-test".to_string(),
            Some("updated@example.com".to_string()),
            Some("Updated User".to_string()),
            None,
            true,
        )
        .expect("upsert update should succeed");

    assert_eq!(updated_user.email, "updated@example.com");
    assert_eq!(updated_user.name, "Updated User");
    // Same user_id because same firebase_uid
    assert_eq!(updated_user.user_id, user.user_id);
}

#[actix_rt::test]
async fn upsert_firebase_user_missing_email_errors() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let user_ops = UserOperations::new(pool.clone()).await;

    let result = user_ops.upsert_firebase_user(
        "firebase-uid-no-email".to_string(),
        None, // no email
        Some("Some User".to_string()),
        None,
        false,
    );

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        RepositoryError::ValidationError(_)
    ));
}

#[actix_rt::test]
async fn create_user_success() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let user_ops = UserOperations::new(pool.clone()).await;

    let user = user_ops
        .upsert_firebase_user(
            "uid-create-test-1".to_string(),
            Some("create_test@example.com".to_string()),
            Some("Create Test".to_string()),
            None,
            true,
        )
        .expect("create user should succeed");

    assert_eq!(user.email, "create_test@example.com");
    assert_eq!(user.firebase_uid, "uid-create-test-1");
}

#[actix_rt::test]
async fn create_user_duplicate_email_errors() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");

    // Insert first user with a specific email
    insert_user(
        conn.connection(),
        "uid-dup-1",
        "dup_email@example.com",
        "User One",
        None,
    )
    .expect("first insert should succeed");

    // Attempt to insert a second user with the same email but different uid
    let result = insert_user(
        conn.connection(),
        "uid-dup-2",
        "dup_email@example.com",
        "User Two",
        None,
    );

    assert!(
        result.is_err(),
        "inserting duplicate email should fail with DB error"
    );
}

#[actix_rt::test]
async fn get_user_by_email_success() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let user_ops = UserOperations::new(pool.clone()).await;

    insert_user(
        conn.connection(),
        "uid-email-lookup",
        "lookup@example.com",
        "Lookup User",
        None,
    )
    .expect("insert user");

    let user = user_ops
        .get_user_by_email("lookup@example.com")
        .expect("get_user_by_email should succeed");

    assert_eq!(user.email, "lookup@example.com");
    assert_eq!(user.firebase_uid, "uid-email-lookup");
}

#[actix_rt::test]
async fn get_user_by_email_not_found() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let user_ops = UserOperations::new(pool.clone()).await;

    let result = user_ops.get_user_by_email("nobody@example.com");

    assert!(matches!(result, Err(RepositoryError::NotFound(_))));
}

#[actix_rt::test]
async fn get_past_orders_by_userid_empty() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let user_ops = UserOperations::new(pool.clone()).await;

    let new_user_id = insert_user(
        conn.connection(),
        "uid-no-orders",
        "no_orders@example.com",
        "No Orders User",
        None,
    )
    .expect("insert user");

    let orders = user_ops
        .get_past_orders_by_userid(&new_user_id)
        .await
        .expect("get past orders");

    assert!(orders.is_empty(), "new user should have no past orders");
}

#[actix_rt::test]
async fn get_past_orders_by_userid_populated() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let user_ops = UserOperations::new(pool.clone()).await;
    let order_ops = OrderOperations::new(pool.clone()).await;

    // Insert a fresh user to avoid cross-test interference
    let user_id_val = insert_user(
        conn.connection(),
        "uid-has-orders",
        "has_orders@example.com",
        "Has Orders User",
        None,
    )
    .expect("insert user");

    // Create and deliver an order for the new user
    order_ops
        .create_order(user_id_val, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    use diesel::prelude::*;
    use proj_xs::db::DbConnection;
    let order_id_val: i32 = {
        let mut c = DbConnection::new(&pool).expect("conn");
        use proj_xs::db::schema::active_orders::dsl::*;
        active_orders
            .filter(user_id.eq(user_id_val))
            .select(order_id)
            .first(c.connection())
            .expect("order id")
    };

    order_ops
        .order_actions(&order_id_val, "delivered")
        .expect("deliver order");

    let orders = user_ops
        .get_past_orders_by_userid(&user_id_val)
        .await
        .expect("get past orders");

    assert_eq!(orders.len(), 1, "should have exactly one past order");
    assert!(orders[0].order_status, "order should be marked delivered");
}
