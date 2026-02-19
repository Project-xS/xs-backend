mod common;

use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use diesel::prelude::*;
use proj_xs::db::{DbConnection, OrderOperations};
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

#[actix_rt::test]
async fn get_past_orders_empty() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

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
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert!(data.is_empty(), "no past orders yet");
}

#[actix_rt::test]
async fn get_past_orders_after_delivery() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;

    // Create an order
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    // Retrieve the order id
    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    // Mark the order delivered (moves it to past_orders)
    order_ops
        .order_actions(&order_id_val, "delivered")
        .expect("deliver order");

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
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 1, "should have exactly one past order");
    assert!(data[0]["order_status"].as_bool().unwrap_or(false));
}

#[actix_rt::test]
async fn admin_cannot_get_past_orders() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/users/get_past_orders?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn get_past_orders_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/users/get_past_orders")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
