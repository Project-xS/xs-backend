mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use diesel::prelude::*;
use proj_xs::db::DbConnection;
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

fn canteen_login_credentials(db_url: &str, canteen_id_val: i32) -> (String, String) {
    let pool = build_test_pool(db_url);
    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::canteens::dsl::*;
    let name_val: String = canteens
        .filter(canteen_id.eq(canteen_id_val))
        .select(canteen_name)
        .first::<String>(conn.connection())
        .expect("canteen name");
    let username_val = name_val.to_lowercase().replace(' ', "_");
    let password_val = format!("{username_val}@{:02}", canteen_id_val);
    (username_val, password_val)
}

#[actix_rt::test]
async fn canteen_login_success() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let (username, password) = canteen_login_credentials(&db_url, fixtures.canteen_id);

    let req = test::TestRequest::post()
        .uri("/canteen/login")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "username": username,
            "password": password
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["token"].is_string());
    assert_eq!(body["data"]["canteen_name"], "Test Canteen");
}

#[actix_rt::test]
async fn canteen_login_wrong_password() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let (username, _password) = canteen_login_credentials(&db_url, fixtures.canteen_id);

    let req = test::TestRequest::post()
        .uri("/canteen/login")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "username": username,
            "password": "wrong_password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "invalid_credentials");
}

#[actix_rt::test]
async fn canteen_login_unknown_user() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/canteen/login")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "username": "nonexistent_canteen",
            "password": "nonexistent_canteen@01"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_all_canteens_returns_seeded() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/canteen")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert!(!data.is_empty(), "should have at least the seeded canteen");
    assert_eq!(data[0]["canteen_name"], "Test Canteen");
}

#[actix_rt::test]
async fn get_canteen_items_as_user() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/canteen/{}/items?as=user-{}",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 2, "should return the 2 seeded items");
}

#[actix_rt::test]
async fn get_canteen_items_as_admin_own_canteen() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Admin ignores path id, uses own canteen_id from principal
    let req = test::TestRequest::get()
        .uri(&format!(
            "/canteen/{}/items?as=admin-{}",
            fixtures.canteen_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 2);
}

#[actix_rt::test]
async fn user_cannot_create_canteen() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "canteen_name": "Unauthorized Canteen",
            "location": "Block Z",
            "has_pic": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn create_canteen_as_admin_success() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "canteen_name": "New Canteen",
            "location": "Block Z",
            "has_pic": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

#[actix_rt::test]
async fn upload_and_set_canteen_pic_paths() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/canteen/upload_pic?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["presigned_url"].is_string());

    let req = test::TestRequest::put()
        .uri(&format!(
            "/canteen/set_pic?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn get_canteen_items_nonexistent_canteen() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/canteen/99999/items?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert!(
        data.is_empty(),
        "non-existent canteen should return empty items"
    );
}

#[actix_rt::test]
async fn canteen_login_requires_content_type() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/canteen/login")
        .insert_header(auth_header())
        .set_payload(r#"{"username":"test","password":"test"}"#)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn create_canteen_requires_content_type() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .set_payload(r#"{"canteen_name":"X","location":"Y","has_pic":false}"#)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_rt::test]
async fn create_canteen_malformed_json() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{not valid json}")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn create_canteen_missing_required_fields() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({"canteen_name": "Only Name"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn create_canteen_whitespace_name() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "canteen_name": "   ",
            "location": "Block A",
            "has_pic": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn create_canteen_duplicate() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let payload = serde_json::json!({
        "canteen_name": "Duplicate Canteen",
        "location": "Block D",
        "has_pic": false
    });
    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req2 = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&payload)
        .to_request();
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn create_canteen_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/canteen/create")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "canteen_name": "Ghost Canteen",
            "location": "Block G",
            "has_pic": false
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
async fn canteen_login_malformed_json() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/canteen/login")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_payload("{bad json}")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn canteen_login_missing_fields() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/canteen/login")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({"username": "only_username"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn upload_canteen_pic_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/canteen/upload_pic?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn upload_canteen_pic_nonexistent_canteen() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri("/canteen/upload_pic?as=admin-99999")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn set_canteen_pic_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri(&format!("/canteen/set_pic?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn set_canteen_pic_nonexistent_canteen() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri("/canteen/set_pic?as=admin-99999")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[actix_rt::test]
async fn get_all_canteens_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get().uri("/canteen").to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_all_canteens_has_pic_true_null_link() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Create a canteen with has_pic = true
    let req = test::TestRequest::post()
        .uri(&format!("/canteen/create?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "canteen_name": "Pic Canteen",
            "location": "Block P",
            "has_pic": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // List all canteens — pic_link should be null in test env (no S3 object)
    let req = test::TestRequest::get()
        .uri("/canteen")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let data = body["data"].as_array().expect("data array");
    let pic_canteen = data
        .iter()
        .find(|c| c["canteen_name"] == "Pic Canteen")
        .expect("Pic Canteen present");
    assert!(pic_canteen["pic_link"].is_null());
}

#[actix_rt::test]
async fn get_canteen_items_admin_different_canteen() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Admin requests items for canteen_id 99999 — should still return admin's own canteen items
    let req = test::TestRequest::get()
        .uri(&format!(
            "/canteen/99999/items?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert_eq!(data.len(), 2, "should return admin's own canteen items");
}

#[actix_rt::test]
async fn get_canteen_items_unauthenticated() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/canteen/{}/items", fixtures.canteen_id))
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn upload_canteen_pic_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri("/canteen/upload_pic")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn set_canteen_pic_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::put()
        .uri("/canteen/set_pic")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn canteen_set_pic_success_when_object_exists() {
    // Mock S3 before app creation.
    let mock_s3 = common::start_mock_s3().await;
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // set_canteen_pic calls get_object_etag("canteens/{canteen_id}")
    mock_s3
        .mock_object_exists(&format!("canteens/{}", fixtures.canteen_id))
        .await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/canteen/set_pic?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(common::auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}
