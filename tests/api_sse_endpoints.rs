mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;

#[actix_rt::test]
async fn user_order_events_stream_connects_for_user() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/users/events/orders?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type")
        .to_str()
        .expect("header str");
    assert!(content_type.contains("text/event-stream"));

    let mut body = resp.into_body();
    let retry = common::read_sse_frame(&mut body).await;
    assert_eq!(retry.retry_ms, Some(3000));

    let connected = common::wait_for_connected_event(&mut body).await;
    assert_eq!(connected.event.as_deref(), Some("status"));
    assert_eq!(connected.data.as_deref(), Some("connected"));
}

#[actix_rt::test]
async fn canteen_aggregated_events_stream_connects_for_admin() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/canteen/events/orders?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type")
        .to_str()
        .expect("header str");
    assert!(content_type.contains("text/event-stream"));

    let mut body = resp.into_body();
    let retry = common::read_sse_frame(&mut body).await;
    assert_eq!(retry.retry_ms, Some(3000));

    let connected = common::wait_for_connected_event(&mut body).await;
    assert_eq!(connected.event.as_deref(), Some("status"));
    assert_eq!(connected.data.as_deref(), Some("connected"));
}

#[actix_rt::test]
async fn inventory_events_stream_connects_for_user() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/menu/events/inventory/{}?as=user-{}",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type")
        .to_str()
        .expect("header str");
    assert!(content_type.contains("text/event-stream"));

    let mut body = resp.into_body();
    let retry = common::read_sse_frame(&mut body).await;
    assert_eq!(retry.retry_ms, Some(3000));

    let connected = common::wait_for_connected_event(&mut body).await;
    assert_eq!(connected.event.as_deref(), Some("status"));
    assert_eq!(connected.data.as_deref(), Some("connected"));
}

#[actix_rt::test]
async fn inventory_events_stream_connects_for_admin() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/menu/events/inventory/{}?as=admin-{}",
            fixtures.canteen_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let mut body = resp.into_body();
    let _retry = common::read_sse_frame(&mut body).await;
    let connected = common::wait_for_connected_event(&mut body).await;
    assert_eq!(connected.event.as_deref(), Some("status"));
    assert_eq!(connected.data.as_deref(), Some("connected"));
}

#[actix_rt::test]
async fn user_order_events_forbid_admin_role() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/users/events/orders?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn canteen_events_forbid_user_role() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/canteen/events/orders?as=user-{}",
            fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn user_order_events_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;
    let req = test::TestRequest::get()
        .uri("/users/events/orders")
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn canteen_events_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;
    let req = test::TestRequest::get()
        .uri("/canteen/events/orders")
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn inventory_events_unauthenticated() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;
    let req = test::TestRequest::get()
        .uri(&format!("/menu/events/inventory/{}", fixtures.canteen_id))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}
