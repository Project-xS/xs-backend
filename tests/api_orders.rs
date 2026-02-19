mod common;

use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use diesel::prelude::*;
use proj_xs::db::{DbConnection, OrderOperations};
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

#[actix_rt::test]
async fn put_order_actions_and_invalid_action() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;

    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order 1");
    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/delivered?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[1]], None)
        .expect("create order 2");
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/cancelled?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/invalid?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn get_orders_by_user_admin_and_user() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&user_id={}",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&user_id={}&rfid=rfid-1",
            fixtures.canteen_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_rt::test]
async fn get_all_orders_admin_returns_counts() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let req = test::TestRequest::get()
        .uri(&format!("/orders?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["data"].is_object(), "data should be a time-band map");
}

#[actix_rt::test]
async fn get_order_by_id_admin() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/{}?as=admin-{}",
            order_id_val, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
}

#[actix_rt::test]
async fn user_cannot_get_all_orders() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/orders?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn user_cannot_order_actions() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let mut conn = DbConnection::new(&pool).expect("db connection");
    use proj_xs::db::schema::active_orders::dsl::*;
    let order_id_val = active_orders
        .select(order_id)
        .first::<i32>(conn.connection())
        .expect("order id");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/orders/{}/delivered?as=user-{}",
            order_id_val, fixtures.user_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn get_orders_by_user_admin_rfid_and_missing_params() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let pool = build_test_pool(&db_url);
    let order_ops = OrderOperations::new(pool.clone()).await;
    order_ops
        .create_order(fixtures.user_id, vec![fixtures.menu_item_ids[0]], None)
        .expect("create order");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/orders/by_user?as=admin-{}&rfid=rfid-1",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::get()
        .uri(&format!("/orders/by_user?as=admin-{}", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
