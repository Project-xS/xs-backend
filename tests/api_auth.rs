mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;

#[actix_rt::test]
async fn rejects_missing_bearer_token() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=user-{}", fixtures.user_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn rejects_invalid_bearer_token() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=user-{}", fixtures.user_id))
        .insert_header((header::AUTHORIZATION, "Bearer invalid-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
