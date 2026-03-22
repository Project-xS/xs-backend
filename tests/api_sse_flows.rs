mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use diesel::prelude::*;
use proj_xs::db::{DbConnection, OrderOperations};
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

fn assert_numeric_sse_id(frame: &common::SseFrame) {
    let id = frame.id.as_deref().expect("SSE event id should be present");
    assert!(
        id.parse::<u128>().is_ok(),
        "SSE id should be numeric millis"
    );
}

fn find_item_payload(payload: &Value, item_id_val: i32) -> Value {
    payload["items"]
        .as_array()
        .expect("items array")
        .iter()
        .find(|item| item["item_id"].as_i64() == Some(item_id_val as i64))
        .expect("expected item payload")
        .clone()
}

#[actix_rt::test]
async fn hold_order_emits_inventory_update_to_inventory_stream() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let item_id = fixtures.menu_item_ids[0];
    let (initial_stock, _) = common::menu_item_state(conn.connection(), item_id);
    let expected_stock = if initial_stock == -1 {
        -1
    } else {
        initial_stock - 1
    };
    let expected_available = expected_stock > 0 || expected_stock == -1;

    let sse_req = test::TestRequest::get()
        .uri(&format!(
            "/menu/events/inventory/{}?as=user-{}",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let sse_resp = test::call_service(&app, sse_req).await;
    assert_eq!(sse_resp.status(), StatusCode::OK);
    let mut inventory_stream = sse_resp.into_body();
    let _retry = common::read_sse_frame(&mut inventory_stream).await;
    let _connected = common::wait_for_connected_event(&mut inventory_stream).await;

    let hold_req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": null,
            "item_ids": [item_id]
        }))
        .to_request();
    let hold_resp = test::call_service(&app, hold_req).await;
    assert_eq!(hold_resp.status(), StatusCode::OK);

    let event = common::wait_for_sse_event(&mut inventory_stream, "inventory_update").await;
    assert_numeric_sse_id(&event);
    let payload = common::sse_frame_data_json(&event);
    let item_payload = find_item_payload(&payload, item_id);
    assert_eq!(item_payload["stock"], expected_stock);
    assert_eq!(item_payload["is_available"], expected_available);
    assert!(item_payload["price"].is_number());
}

#[actix_rt::test]
async fn confirm_hold_emits_user_and_canteen_sse_events() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;
    let item_id = fixtures.menu_item_ids[0];
    let deliver_at = "11:00am - 12:00pm";

    let user_sse_req = test::TestRequest::get()
        .uri(&format!(
            "/users/events/orders?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let user_sse_resp = test::call_service(&app, user_sse_req).await;
    assert_eq!(user_sse_resp.status(), StatusCode::OK);
    let mut user_stream = user_sse_resp.into_body();
    let _user_retry = common::read_sse_frame(&mut user_stream).await;
    let _user_connected = common::wait_for_connected_event(&mut user_stream).await;

    let canteen_sse_req = test::TestRequest::get()
        .uri(&format!(
            "/canteen/events/orders?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let canteen_sse_resp = test::call_service(&app, canteen_sse_req).await;
    assert_eq!(canteen_sse_resp.status(), StatusCode::OK);
    let mut canteen_stream = canteen_sse_resp.into_body();
    let _canteen_retry = common::read_sse_frame(&mut canteen_stream).await;
    let _canteen_connected = common::wait_for_connected_event(&mut canteen_stream).await;

    let hold_req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": deliver_at,
            "item_ids": [item_id]
        }))
        .to_request();
    let hold_resp = test::call_service(&app, hold_req).await;
    assert_eq!(hold_resp.status(), StatusCode::OK);
    let hold_body: Value = test::read_body_json(hold_resp).await;
    let hold_id = hold_body["hold_id"].as_i64().expect("hold id");

    let confirm_req = test::TestRequest::post()
        .uri(&format!(
            "/orders/hold/{}/confirm?as=admin-{}",
            hold_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let confirm_resp = test::call_service(&app, confirm_req).await;
    assert_eq!(confirm_resp.status(), StatusCode::OK);
    let confirm_body: Value = test::read_body_json(confirm_resp).await;
    let order_id = confirm_body["order_id"].as_i64().expect("order_id");

    let user_event = common::wait_for_sse_event(&mut user_stream, "user_order_update").await;
    assert_numeric_sse_id(&user_event);
    let user_payload = common::sse_frame_data_json(&user_event);
    assert_eq!(user_payload["order_id"], order_id);
    assert_eq!(user_payload["status"], "placed");

    let canteen_event =
        common::wait_for_sse_event(&mut canteen_stream, "canteen_aggregated_order_update").await;
    assert_numeric_sse_id(&canteen_event);
    let canteen_payload = common::sse_frame_data_json(&canteen_event);
    assert_eq!(canteen_payload["time_band"], deliver_at);

    let item_payload = find_item_payload(&canteen_payload, item_id);
    assert!(item_payload["num_ordered"].as_i64().unwrap_or_default() >= 1);
}

#[actix_rt::test]
async fn order_actions_emit_user_order_update_events() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    let mut conn = DbConnection::new(&pool).expect("db connection");

    let user_sse_req = test::TestRequest::get()
        .uri(&format!(
            "/users/events/orders?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let user_sse_resp = test::call_service(&app, user_sse_req).await;
    assert_eq!(user_sse_resp.status(), StatusCode::OK);
    let mut user_stream = user_sse_resp.into_body();
    let _retry = common::read_sse_frame(&mut user_stream).await;
    let _connected = common::wait_for_connected_event(&mut user_stream).await;

    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order for delivered");

    use proj_xs::db::schema::active_orders::dsl as ao;
    let delivered_order_id: i32 = ao::active_orders
        .select(ao::order_id)
        .order(ao::order_id.desc())
        .first(conn.connection())
        .expect("latest order id");

    let delivered_req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/delivered?as=admin-{}",
            delivered_order_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let delivered_resp = test::call_service(&app, delivered_req).await;
    assert_eq!(delivered_resp.status(), StatusCode::OK);

    let delivered_event = common::wait_for_sse_event(&mut user_stream, "user_order_update").await;
    assert_numeric_sse_id(&delivered_event);
    let delivered_payload = common::sse_frame_data_json(&delivered_event);
    assert_eq!(delivered_payload["order_id"], delivered_order_id);
    assert_eq!(delivered_payload["status"], "delivered");

    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[1]], None)
        .expect("create order for cancelled");
    let cancelled_order_id: i32 = ao::active_orders
        .select(ao::order_id)
        .order(ao::order_id.desc())
        .first(conn.connection())
        .expect("latest order id");

    let cancelled_req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/cancelled?as=admin-{}",
            cancelled_order_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let cancelled_resp = test::call_service(&app, cancelled_req).await;
    assert_eq!(cancelled_resp.status(), StatusCode::OK);

    let cancelled_event = common::wait_for_sse_event(&mut user_stream, "user_order_update").await;
    assert_numeric_sse_id(&cancelled_event);
    let cancelled_payload = common::sse_frame_data_json(&cancelled_event);
    assert_eq!(cancelled_payload["order_id"], cancelled_order_id);
    assert_eq!(cancelled_payload["status"], "cancelled");
}

#[actix_rt::test]
async fn update_menu_item_emits_inventory_update_event() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;
    let item_id = fixtures.menu_item_ids[0];

    let sse_req = test::TestRequest::get()
        .uri(&format!(
            "/menu/events/inventory/{}?as=admin-{}",
            fixtures.canteen_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let sse_resp = test::call_service(&app, sse_req).await;
    assert_eq!(sse_resp.status(), StatusCode::OK);
    let mut inventory_stream = sse_resp.into_body();
    let _retry = common::read_sse_frame(&mut inventory_stream).await;
    let _connected = common::wait_for_connected_event(&mut inventory_stream).await;

    let update_req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": item_id,
            "update": {
                "price": 222,
                "stock": 4,
                "is_available": true
            }
        }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let event = common::wait_for_sse_event(&mut inventory_stream, "inventory_update").await;
    assert_numeric_sse_id(&event);
    let payload = common::sse_frame_data_json(&event);
    let item_payload = find_item_payload(&payload, item_id);
    assert_eq!(item_payload["stock"], 4);
    assert_eq!(item_payload["is_available"], true);
    assert_eq!(item_payload["price"], 222);
}
