mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use base64::Engine;
use common::auth_header;
use diesel::prelude::*;
use hmac::Mac;
use proj_xs::auth::qr_token;
use proj_xs::db::{DbConnection, OrderOperations};
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

#[actix_rt::test]
async fn qr_generation_owner_not_owner_and_not_found() {
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
            "/orders/{}/qr?as=user-{}",
            order_id_val, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("image/png"));

    let req = test::TestRequest::get()
        .uri(&format!("/orders/{}/qr?as=user-2", order_id_val))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let req = test::TestRequest::get()
        .uri("/orders/9999/qr?as=user-1")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn scan_qr_success_invalid_and_canteen_mismatch() {
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

    let secret = std::env::var("DELIVER_QR_HASH_SECRET").expect("DELIVER_QR_HASH_SECRET");
    let token = qr_token::generate_qr_token(order_id_val, fixtures.user_id, &secret);

    let req = test::TestRequest::post()
        .uri(&format!("/orders/scan?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");

    let req = test::TestRequest::post()
        .uri(&format!("/orders/scan?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": "bad-token" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = test::TestRequest::post()
        .uri("/orders/scan?as=admin-999")
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn scan_qr_requires_content_type() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;
    let req = test::TestRequest::post()
        .uri("/orders/scan?as=admin-1")
        .insert_header(auth_header())
        .set_payload(r#"{"token":"x"}"#)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn scan_qr_missing_order_returns_bad_request() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let secret = std::env::var("DELIVER_QR_HASH_SECRET").expect("DELIVER_QR_HASH_SECRET");
    let token = qr_token::generate_qr_token(9999, fixtures.user_id, &secret);

    let req = test::TestRequest::post()
        .uri(&format!("/orders/scan?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn qr_generation_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get().uri("/orders/1/qr").to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn scan_qr_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/orders/scan")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": "anything" }))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn scan_qr_missing_token_field() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/orders/scan?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn scan_qr_expired_token() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let secret = std::env::var("DELIVER_QR_HASH_SECRET").expect("DELIVER_QR_HASH_SECRET");
    // Build a token with timestamp=1 (expired)
    let payload = format!("1|{}|1", fixtures.user_id);
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).expect("valid key");
    hmac::Mac::update(&mut mac, payload.as_bytes());
    let signature = hex::encode(hmac::Mac::finalize(mac).into_bytes());
    let token_raw = format!("{}|{}", payload, signature);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_raw.as_bytes());

    let req = test::TestRequest::post()
        .uri(&format!("/orders/scan?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn scan_qr_already_delivered_order() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    // Create an order directly
    let pool = build_test_pool(&db_url);
    let order_ops = proj_xs::db::OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    // Deliver the order via API
    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/delivered?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Generate a valid QR token for the now-delivered order
    let secret = std::env::var("DELIVER_QR_HASH_SECRET").expect("DELIVER_QR_HASH_SECRET");
    let token = qr_token::generate_qr_token(order_id_val, fixtures.user_id, &secret);

    // Scanning should return 400 since order is no longer active
    let req = test::TestRequest::post()
        .uri(&format!("/orders/scan?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": token }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn admin_cannot_generate_order_qr() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders/1/qr?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn user_cannot_scan_qr() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/orders/scan?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({ "token": "sometoken" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn qr_generation_order_with_no_items_returns_not_found() {
    // Insert an order without items to trigger the empty-items path.
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let mut conn = DbConnection::new(&pool).expect("db connection");

    // Insert an order without items.
    use proj_xs::db::schema::active_orders::dsl as ao;
    let order_id_val: i32 = diesel::insert_into(ao::active_orders)
        .values((
            ao::user_id.eq(fixtures.user_id),
            ao::canteen_id.eq(fixtures.canteen_id),
            ao::total_price.eq(0),
        ))
        .returning(ao::order_id)
        .get_result(conn.connection())
        .expect("insert order without items");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/{}/qr?as=user-{}",
            order_id_val, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}
