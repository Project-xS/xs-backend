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

#[actix_rt::test]
async fn create_menu_item_malformed_json() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{not valid json}")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn create_menu_item_missing_required_fields() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({"name": "Only Name"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn create_menu_item_whitespace_name() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "   ",
            "is_veg": true,
            "price": 50,
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
async fn create_menu_item_overlong_name() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let long_name = "a".repeat(121);
    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": long_name,
            "is_veg": true,
            "price": 50,
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
async fn create_menu_item_negative_price() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Cheap Item",
            "is_veg": true,
            "price": -5,
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
async fn create_menu_item_negative_stock() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Ghost Stock Item",
            "is_veg": false,
            "price": 100,
            "stock": -2,
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
async fn create_menu_item_has_pic_true() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Pic Item",
            "is_veg": true,
            "price": 120,
            "stock": 5,
            "is_available": true,
            "description": "Has a pic",
            "has_pic": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["item_id"].is_number());
}

#[actix_rt::test]
async fn create_menu_item_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/menu/create")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "Ghost",
            "is_veg": true,
            "price": 50,
            "stock": 10,
            "is_available": true,
            "description": null,
            "has_pic": false
        }))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn update_menu_item_malformed_json() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{bad json}")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn update_menu_item_missing_item_id() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({"update": {"name": "NoId"}}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn update_menu_item_all_null_update() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": {}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    // Diesel may treat all-None changeset as no-op (200) or error (409)
    assert!(
        status == StatusCode::OK || status == StatusCode::CONFLICT,
        "expected 200 or 409 for all-null update, got {:?}",
        status
    );
}

#[actix_rt::test]
async fn update_menu_item_overlong_name() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let long_name = "b".repeat(121);
    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": {"name": long_name}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn update_menu_item_overlong_description() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let long_desc = "c".repeat(501);
    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": {"description": long_desc}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn update_menu_item_negative_price() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/menu/update?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": {"price": -5}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn update_menu_item_unauthenticated() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri("/menu/update")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "item_id": fixtures.menu_item_ids[0],
            "update": {"name": "Ghost"}
        }))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn delete_menu_item_unauthenticated() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::delete()
        .uri(&format!("/menu/delete/{}", fixtures.menu_item_ids[0]))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn delete_menu_item_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/menu/delete/{}?as=user-{}",
            fixtures.menu_item_ids[0], fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn upload_menu_item_pic_nonexistent_item() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/menu/upload_pic/99999?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn upload_menu_item_pic_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/menu/upload_pic/{}?as=user-{}",
            fixtures.menu_item_ids[0], fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn set_menu_item_pic_nonexistent_item() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/menu/set_pic/99999?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn set_menu_item_pic_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/menu/set_pic/{}?as=user-{}",
            fixtures.menu_item_ids[0], fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn get_menu_items_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/menu/items?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn get_menu_items_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get().uri("/menu/items").to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn get_menu_item_by_id_unauthenticated() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/menu/items/{}", fixtures.menu_item_ids[0]))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn get_menu_item_has_pic_true() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Create a menu item with has_pic = true
    let create_req = test::TestRequest::post()
        .uri(&format!("/menu/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "name": "With Pic",
            "is_veg": true,
            "price": 100,
            "stock": 10,
            "is_available": true,
            "description": null,
            "has_pic": true
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::OK);
    let create_body: Value = test::read_body_json(create_resp).await;
    let item_id = create_body["item_id"].as_i64().expect("item_id");

    // Fetch the item — pic_link is null since no S3 object exists in the test env
    let get_req = test::TestRequest::get()
        .uri(&format!(
            "/menu/items/{}?as=admin-{}",
            item_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body: Value = test::read_body_json(get_resp).await;
    assert_eq!(get_body["status"], "ok");
    assert!(get_body["data"]["pic_link"].is_null());
}

#[actix_rt::test]
async fn upload_menu_item_pic_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri("/menu/upload_pic/1")
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn set_menu_item_pic_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put().uri("/menu/set_pic/1").to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn menu_set_pic_success_when_object_exists() {
    // Mock S3 before app creation so the embedded AssetOperations uses the mock endpoint.
    let mock_s3 = common::start_mock_s3().await;
    let (app, fixtures, _db_url) = common::setup_api_app().await;
    let item_id = fixtures.menu_item_ids[0];

    // set_menu_item_pic calls get_object_etag("items/{item_id}")
    mock_s3
        .mock_object_exists(&format!("items/{}", item_id))
        .await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/menu/set_pic/{}?as=admin-{}",
            item_id, fixtures.canteen_id
        ))
        .insert_header(common::auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}
