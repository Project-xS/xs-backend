mod common;

use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use serde_json::Value;

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
