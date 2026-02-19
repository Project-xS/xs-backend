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
    // Middleware returns Err for missing token; use try_call_service to capture it
    let status = match test::try_call_service(&app, req).await {
        Ok(resp) => resp.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[actix_rt::test]
async fn rejects_invalid_bearer_token() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=user-{}", fixtures.user_id))
        .insert_header((header::AUTHORIZATION, "Bearer invalid-token"))
        .to_request();
    let status = match test::try_call_service(&app, req).await {
        Ok(resp) => resp.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
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
