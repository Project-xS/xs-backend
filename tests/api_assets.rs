mod common;

use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use diesel::prelude::*;
use proj_xs::db::DbConnection;
use proj_xs::test_utils::{build_test_pool, insert_canteen, seed_menu_item};
use serde_json::Value;

fn set_menu_item_pic_key(db_url: &str, item_id_val: i32, key: &str) {
    let pool = build_test_pool(db_url);
    let mut conn = DbConnection::new(&pool).expect("db connection");

    use proj_xs::db::schema::menu_items::dsl as mi;
    diesel::update(mi::menu_items.filter(mi::item_id.eq(item_id_val)))
        .set(mi::pic_key.eq(Some(key.to_string())))
        .execute(conn.connection())
        .expect("set pic_key");
}

fn seed_other_canteen_item_with_pic_key(db_url: &str, key: &str) -> i32 {
    let pool = build_test_pool(db_url);
    let mut conn = DbConnection::new(&pool).expect("db connection");
    let other_canteen_id =
        insert_canteen(conn.connection(), "Other Canteen", "Block Z").expect("insert canteen");
    let other_item_id = seed_menu_item(
        conn.connection(),
        other_canteen_id,
        "Other Canteen Item",
        199,
        8,
        true,
        true,
        Some("owned by another canteen"),
    )
    .expect("insert menu item");

    use proj_xs::db::schema::menu_items::dsl as mi;
    diesel::update(mi::menu_items.filter(mi::item_id.eq(other_item_id)))
        .set(mi::pic_key.eq(Some(key.to_string())))
        .execute(conn.connection())
        .expect("set other canteen pic_key");
    other_item_id
}

#[actix_rt::test]
async fn get_asset_success_when_object_exists() {
    let mock_s3 = common::start_mock_s3().await;
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let key = "owned-key-get-success";
    set_menu_item_pic_key(&db_url, fixtures.menu_item_ids[0], key);
    mock_s3.mock_object_exists(key).await;

    let req = test::TestRequest::get()
        .uri(&format!("/assets/{}?as=admin-{}", key, fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["url"].as_str().unwrap_or("").starts_with("http"));
}

#[actix_rt::test]
async fn get_asset_not_found_when_owned_key_missing_object() {
    let mock_s3 = common::start_mock_s3().await;
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let key = "owned-key-missing-object";
    set_menu_item_pic_key(&db_url, fixtures.menu_item_ids[0], key);
    mock_s3.mock_object_not_found(key).await;

    let req = test::TestRequest::get()
        .uri(&format!("/assets/{}?as=admin-{}", key, fixtures.canteen_id))
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
async fn asset_get_forbidden_when_key_not_owned() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/assets/not-owned-key?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
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
async fn upload_asset_nonexistent_item_id_forbidden() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::post()
        .uri(&format!(
            "/assets/upload/99999?as=admin-{}",
            fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "error");
    assert_eq!(body["error"], "item not found");
}

#[actix_rt::test]
async fn upload_asset_other_canteen_item_forbidden() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let other_item_id = seed_other_canteen_item_with_pic_key(&db_url, "other-key-upload");

    let req = test::TestRequest::post()
        .uri(&format!(
            "/assets/upload/{}?as=admin-{}",
            other_item_id, fixtures.canteen_id
        ))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[actix_rt::test]
async fn get_asset_other_canteen_key_forbidden() {
    let (app, fixtures, db_url) = common::setup_api_app().await;
    let key = "other-canteen-key";
    let _other_item_id = seed_other_canteen_item_with_pic_key(&db_url, key);

    let req = test::TestRequest::get()
        .uri(&format!("/assets/{}?as=admin-{}", key, fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
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
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn get_asset_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/assets/some_key")
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}
