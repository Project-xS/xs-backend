mod common;

use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use serde_json::Value;

#[actix_rt::test]
async fn root_endpoint_no_auth() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = test::read_body(resp).await;
    assert_eq!(body, "Server up!");
}

#[actix_rt::test]
async fn health_endpoint_no_auth() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = test::read_body(resp).await;
    assert_eq!(body, "OK");
}

#[actix_rt::test]
async fn get_all_canteens_no_role_required() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    // No ?as= parameter required — any authenticated request works
    let req = test::TestRequest::get()
        .uri("/canteen")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

#[actix_rt::test]
async fn get_menu_item_no_role_required() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let item_id = fixtures.menu_item_ids[0];
    // No ?as= parameter required
    let req = test::TestRequest::get()
        .uri(&format!("/menu/items/{}", item_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["data"]["item_id"], item_id);
}

#[actix_rt::test]
async fn search_no_role_required() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    // No ?as= parameter required
    let req = test::TestRequest::get()
        .uri("/search/Veg%20Sandwich")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}
