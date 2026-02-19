mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use diesel::prelude::*;
use proj_xs::db::HoldOperations;
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

#[actix_rt::test]
async fn post_hold_valid_payload_and_invalid_deliver_at() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

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
    assert_eq!(body["status"], "ok");
    assert!(body["hold_id"].is_number());

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": "invalid",
            "item_ids": [fixtures.menu_item_ids[0]]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn post_hold_empty_items_conflict() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": null,
            "item_ids": []
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn post_hold_requires_content_type() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .set_payload(r#"{"deliver_at":null,"item_ids":[1]}"#)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn confirm_hold_success_and_expired() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

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
    let body: Value = test::read_body_json(resp).await;
    let hold_id = body["hold_id"].as_i64().expect("hold id");

    let req = test::TestRequest::post()
        .uri(&format!(
            "/orders/hold/{}/confirm?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");

    let pool = build_test_pool(&db_url);
    let hold_ops = HoldOperations::new(pool.clone(), -1);
    let (expired_id, _) = hold_ops
        .hold_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("expired hold");

    let req = test::TestRequest::post()
        .uri(&format!(
            "/orders/hold/{}/confirm?as=user-{}",
            expired_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn confirm_hold_wrong_owner_conflict() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

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
    let body: Value = test::read_body_json(resp).await;
    let hold_id = body["hold_id"].as_i64().expect("hold id");

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold/{}/confirm?as=user-2", hold_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn delete_hold_success_and_wrong_owner() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

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
    let body: Value = test::read_body_json(resp).await;
    let hold_id = body["hold_id"].as_i64().expect("hold id");

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/orders/hold/{}?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let pool = build_test_pool(&db_url);
    let hold_ops = HoldOperations::new(pool.clone(), 300);
    let (hold_id, _) = hold_ops
        .hold_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("hold order");

    let req = test::TestRequest::delete()
        .uri(&format!("/orders/hold/{}?as=user-2", hold_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn post_hold_with_valid_deliver_at() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": "11:00am - 12:00pm",
            "item_ids": [fixtures.menu_item_ids[0]]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["hold_id"].is_number());
}

#[actix_rt::test]
async fn post_hold_stock_boundary() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    // Set stock to 1
    let pool = build_test_pool(&db_url);
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::menu_items::dsl as mi_dsl;
    diesel::update(mi_dsl::menu_items.filter(mi_dsl::item_id.eq(fixtures.menu_item_ids[0])))
        .set((mi_dsl::stock.eq(1), mi_dsl::is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    // First hold should succeed
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

    // Second hold for the same item should fail (out of stock)
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
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn post_hold_multiple_same_item() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    // Ensure enough stock
    let pool = build_test_pool(&db_url);
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::menu_items::dsl as mi_dsl;
    diesel::update(mi_dsl::menu_items.filter(mi_dsl::item_id.eq(fixtures.menu_item_ids[0])))
        .set((mi_dsl::stock.eq(10), mi_dsl::is_available.eq(true)))
        .execute(conn.connection())
        .expect("set stock");

    let item_id = fixtures.menu_item_ids[0];
    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": null,
            "item_ids": [item_id, item_id, item_id]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");

    // Stock should be decremented by 3
    let (stock_val, _) = common::menu_item_state(conn.connection(), item_id);
    assert_eq!(
        stock_val, 7,
        "stock should be decremented by 3 (10 - 3 = 7)"
    );
}

#[actix_rt::test]
async fn post_hold_cross_canteen_conflict() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    let pool = build_test_pool(&db_url);
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let canteen2_id =
        proj_xs::test_utils::insert_canteen(conn.connection(), "Second Canteen", "Block B")
            .expect("second canteen");
    let item2_id = proj_xs::test_utils::seed_menu_item(
        conn.connection(),
        canteen2_id,
        "Other Item",
        100,
        10,
        true,
        true,
        None,
    )
    .expect("second item");

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": null,
            "item_ids": [fixtures.menu_item_ids[0], item2_id]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn post_hold_unavailable_item_conflict() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    let pool = build_test_pool(&db_url);
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::menu_items::dsl as mi_dsl;
    diesel::update(mi_dsl::menu_items.filter(mi_dsl::item_id.eq(fixtures.menu_item_ids[0])))
        .set(mi_dsl::is_available.eq(false))
        .execute(conn.connection())
        .expect("set unavailable");

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
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn post_hold_out_of_stock_conflict() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    let pool = build_test_pool(&db_url);
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::menu_items::dsl as mi_dsl;
    diesel::update(mi_dsl::menu_items.filter(mi_dsl::item_id.eq(fixtures.menu_item_ids[0])))
        .set((mi_dsl::stock.eq(0), mi_dsl::is_available.eq(false)))
        .execute(conn.connection())
        .expect("set out of stock");

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
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn post_hold_malformed_json() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{bad json}")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn post_hold_missing_item_ids() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn post_hold_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/orders/hold")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": null,
            "item_ids": [1]
        }))
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn confirm_hold_nonexistent_id() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/orders/hold/99999/confirm?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn confirm_hold_already_confirmed() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

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
    let body: Value = test::read_body_json(resp).await;
    let hold_id = body["hold_id"].as_i64().expect("hold id");

    // First confirm succeeds
    let req = test::TestRequest::post()
        .uri(&format!(
            "/orders/hold/{}/confirm?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Second confirm on same hold id should conflict
    let req = test::TestRequest::post()
        .uri(&format!(
            "/orders/hold/{}/confirm?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn confirm_hold_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/orders/hold/1/confirm")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn delete_hold_nonexistent_id() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::delete()
        .uri(&format!("/orders/hold/99999?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn delete_hold_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::delete()
        .uri("/orders/hold/1")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
