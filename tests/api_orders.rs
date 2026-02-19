mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use diesel::prelude::*;
use proj_xs::db::{DbConnection, OrderOperations};
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

#[actix_rt::test]
async fn put_order_actions_and_invalid_action() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;

    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order 1");
    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/delivered?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[1]], None)
        .expect("create order 2");
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/cancelled?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/invalid?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn get_orders_by_user_admin_and_user() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&user_id={}",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&user_id={}&rfid=rfid-1",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn get_all_orders_admin_returns_counts() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let req = test::TestRequest::get()
        .uri(&format!("/orders?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["data"].is_object(), "data should be a time-band map");
}

#[actix_rt::test]
async fn get_order_by_id_admin() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
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

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/{}?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

#[actix_rt::test]
async fn user_cannot_get_all_orders() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn user_cannot_order_actions() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
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

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/delivered?as=user-{}",
            order_id_val, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn get_orders_by_user_admin_rfid_and_missing_params() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&rfid=rfid-1",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn get_order_by_id_not_found() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders/99999?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(
        body["data"].is_null(),
        "non-existent order should return null data"
    );
}

#[actix_rt::test]
async fn order_actions_nonexistent_order() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/99999/delivered?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn get_orders_by_user_admin_both_params() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Providing both user_id and rfid should return BAD_REQUEST
    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&user_id={}&rfid=rfid-1",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn past_orders_after_cancellation() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;

    // Create an active order
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    // Cancel the order via API
    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/cancelled?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Check past orders
    let req = test::TestRequest::get()
        .uri(&format!(
            "/users/get_past_orders?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 1, "should have one past order");
    assert!(
        !data[0]["order_status"].as_bool().unwrap_or(true),
        "cancelled order should have order_status: false"
    );
}

#[actix_rt::test]
async fn full_lifecycle_hold_confirm_deliver_past_orders() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);

    // Step 1: Create hold via API
    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": null,
            "item_ids": [fixtures.menu_item_ids[0]]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let hold_id = body["hold_id"].as_i64().expect("hold id");

    // Step 2: Confirm hold
    let req = test::TestRequest::post()
        .uri(&format!(
            "/orders/hold/{}/confirm?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Step 3: Get active order id
    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    // Step 4: Deliver via API
    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/delivered?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Step 5: Verify in past orders
    let req = test::TestRequest::get()
        .uri(&format!(
            "/users/get_past_orders?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 1, "should have one past order");
    assert!(
        data[0]["order_status"].as_bool().unwrap_or(false),
        "delivered order should have order_status: true"
    );
}

#[actix_rt::test]
async fn get_all_orders_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get().uri("/orders").to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_orders_by_user_no_orders() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Admin queries a user that exists but has placed no orders
    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&user_id={}",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data should be an array");
    assert!(
        data.is_empty(),
        "user with no orders should return empty list"
    );
}

#[actix_rt::test]
async fn get_orders_by_user_unknown_user_id() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&user_id=99999",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data should be an array");
    assert!(data.is_empty(), "unknown user should return empty list");
}

#[actix_rt::test]
async fn get_orders_by_user_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/orders/by_user?user_id=1")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_order_by_id_user_forbidden() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    let pool = build_test_pool(&db_url);
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

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/{}?as=user-{}",
            order_id_val, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn get_order_by_id_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get().uri("/orders/1").to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn order_actions_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri("/orders/1/delivered")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_orders_by_user_unknown_rfid_empty() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&rfid=nonexistent-rfid-xyz",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data should be an array");
    assert!(
        data.is_empty(),
        "unknown rfid should return empty order list"
    );
}
