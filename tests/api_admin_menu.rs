mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use serde_json::Value;

#[actix_rt::test]
async fn create_menu_item_success() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Masala Dosa",
            "is_veg": true,
            "price": 80,
            "stock": 20,
            "is_available": true,
            "description": "Crispy dosa with potato filling",
            "has_pic": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["item_id"].is_number());
}

#[actix_rt::test]
async fn create_menu_item_validation_rejects_bad_price() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Free Item",
            "is_veg": true,
            "price": 0,
            "stock": 10,
            "is_available": true,
            "description": null,
            "has_pic": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn create_menu_item_requires_content_type() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/menu/create?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .set_payload(r#"{"name":"X","is_veg":true,"price":10,"stock":5,"is_available":true,"description":null,"has_pic":false}"#)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn update_menu_item_success() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": {
                "name": "Updated Sandwich",
                "price": 150
            }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

#[actix_rt::test]
async fn update_menu_item_not_found() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": 99999,
            "update": {
                "name": "Ghost Item"
            }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn delete_menu_item_success_and_not_found() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Delete existing item
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/menu/delete/{}?as=admin-{}",
            fixtures.menu_item_ids[0], fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");

    // Delete non-existent item
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/menu/delete/99999?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn get_all_menu_items_returns_seeded() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/menu/items?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert!(data.len() >= 2, "should have at least the 2 seeded items");
}

#[actix_rt::test]
async fn get_menu_item_by_id_and_not_found() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Existing item
    let req = test::TestRequest::get()
        .uri(&format!(
            "/menu/items/{}?as=admin-{}",
            fixtures.menu_item_ids[0], fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["data"]["item_id"], fixtures.menu_item_ids[0]);

    // Non-existent item
    let req = test::TestRequest::get()
        .uri(&format!(
            "/menu/items/99999?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn user_cannot_create_menu_item() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Unauthorized Item",
            "is_veg": true,
            "price": 50,
            "stock": 10,
            "is_available": true,
            "description": null,
            "has_pic": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn upload_menu_item_pic_presign_success() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/menu/upload_pic/{}?as=admin-{}",
            fixtures.menu_item_ids[0], fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["presigned_url"].is_string());
}

#[actix_rt::test]
async fn set_menu_item_pic_conflict_without_object() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/menu/set_pic/{}?as=admin-{}",
            fixtures.menu_item_ids[0], fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn update_menu_item_rejects_zero_price() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": { "price": 0 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn update_menu_item_rejects_bad_stock() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": { "stock": -2 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn update_menu_item_rejects_empty_name() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": { "name": "" }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn create_menu_item_rejects_long_description() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let long_desc = "a".repeat(501);
    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Valid Item",
            "is_veg": true,
            "price": 100,
            "stock": 10,
            "is_available": true,
            "description": long_desc,
            "has_pic": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn update_menu_item_requires_content_type() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .set_payload(r#"{"item_id":1,"update":{"name":"X"}}"#)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
