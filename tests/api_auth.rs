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
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn rejects_invalid_bearer_token() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=user-{}", fixtures.user_id))
        .insert_header((header::AUTHORIZATION, "Bearer invalid-token"))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn admin_on_user_endpoint_returns_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/users/get_past_orders?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header((
            header::AUTHORIZATION,
            format!(
                "Bearer {}",
                std::env::var("DEV_BYPASS_TOKEN").expect("DEV_BYPASS_TOKEN")
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn dev_bypass_invalid_as_param_defaults_to_user() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    // "?as=garbage" doesn't match "user-{id}" or "admin-{id}"; defaults to user_id=1.
    let req = test::TestRequest::get()
        .uri("/users/get_past_orders?as=garbage")
        .insert_header(common::auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should succeed (200) — defaulted to user_id=1 principal
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn user_on_admin_endpoint_returns_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders?as=user-{}", fixtures.user_id))
        .insert_header((
            header::AUTHORIZATION,
            format!(
                "Bearer {}",
                std::env::var("DEV_BYPASS_TOKEN").expect("DEV_BYPASS_TOKEN")
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
