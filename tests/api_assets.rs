mod common;

use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use serde_json::Value;

#[actix_rt::test]
async fn get_asset_success_when_object_exists() {
    // Mock S3 must be started BEFORE setup_api_app so AssetOperations picks up the endpoint.
    let mock_s3 = common::start_mock_s3().await;
    let (app, fixtures, _db_url) = common::setup_api_app().await;
    let item_id = fixtures.menu_item_ids[0];

    // GET /assets/{key} calls get_object_presign which first calls get_object_etag.
    // The handler uses the raw item_id as the S3 key (no prefix).
    mock_s3.mock_object_exists(&item_id.to_string()).await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/assets/{}?as=admin-{}",
            item_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["url"].as_str().unwrap_or("").starts_with("http"));
}

#[actix_rt::test]
async fn get_asset_not_found_when_key_missing() {
    let mock_s3 = common::start_mock_s3().await;
    let (app, fixtures, _db_url) = common::setup_api_app().await;
    let item_id = fixtures.menu_item_ids[0];

    mock_s3.mock_object_not_found(&item_id.to_string()).await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/assets/{}?as=admin-{}",
            item_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn asset_upload_presign_success() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/assets/upload/{}?as=admin-{}",
            fixtures.menu_item_ids[0], fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["url"].as_str().unwrap_or("").starts_with("http"));
}

#[actix_rt::test]
async fn asset_get_returns_error_without_object() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/assets/{}?as=admin-{}",
            fixtures.menu_item_ids[0], fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_server_error());
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
}

#[actix_rt::test]
async fn upload_asset_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/assets/upload/{}?as=user-{}",
            fixtures.menu_item_ids[0], fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn upload_asset_nonexistent_item_id_still_presigns() {
    // The handler does not check whether item_id exists in the DB;
    // it only generates a presigned S3 URL using the raw id as a key.
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/assets/upload/99999?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

#[actix_rt::test]
async fn get_asset_user_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/assets/{}?as=user-{}",
            fixtures.menu_item_ids[0], fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn upload_asset_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri("/assets/upload/1")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn get_asset_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/assets/some_key")
        .to_request();
    let result = test::try_call_service(&app, req).await;
    let status = match result {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
