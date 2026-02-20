mod common;

use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;

#[actix_rt::test]
async fn search_returns_matching_items() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    // "Veg Sandwich" is seeded; use the exact name to match regardless of pg_trgm defaults.
    let req = test::TestRequest::get()
        .uri("/search/Veg%20Sandwich")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert!(
        !data.is_empty(),
        "search for 'Sandwich' should match 'Veg Sandwich'"
    );
    let names: Vec<&str> = data
        .iter()
        .filter_map(|item| item["name"].as_str())
        .collect();
    assert!(
        names.iter().any(|n| n == &"Veg Sandwich"),
        "result names should include the seeded item"
    );
}

#[actix_rt::test]
async fn search_no_match_returns_empty() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/search/ZZZZZZZZZ")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let resp: Value = test::read_body_json(resp).await;
    assert_eq!(resp["status"], "ok");
    let data = resp["data"].as_array().expect("data array");
    assert!(data.is_empty(), "no items should match 'ZZZZZZZZZ'");
}

#[actix_rt::test]
async fn search_by_canteen_filters_correctly() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    // Search scoped to the seeded canteen
    let req = test::TestRequest::get()
        .uri(&format!("/search/{}/Veg%20Sandwich", fixtures.canteen_id))
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert!(
        !data.is_empty(),
        "canteen-scoped search should match 'Veg Sandwich'"
    );

    // All returned items must belong to the correct canteen
    for item in data {
        assert_eq!(
            item["canteen_id"].as_i64(),
            Some(fixtures.canteen_id as i64),
            "all results should belong to the searched canteen"
        );
    }
}

#[actix_rt::test]
async fn search_unauthenticated() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/search/Sandwich")
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}

#[actix_rt::test]
async fn search_matches_across_multiple_canteens() {
    let (app, fixtures, db_url) = common::setup_api_app().await;

    // Create a second canteen with an item whose name matches "Veg"
    let pool = build_test_pool(&db_url);
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let canteen2_id =
        proj_xs::test_utils::insert_canteen(conn.connection(), "East Canteen", "Block E")
            .expect("second canteen");
    proj_xs::test_utils::seed_menu_item(
        conn.connection(),
        canteen2_id,
        "Veg Burger",
        90,
        10,
        true,
        true,
        None,
    )
    .expect("second canteen item");

    let req = test::TestRequest::get()
        .uri("/search/Veg")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    let canteen_ids: Vec<i64> = data
        .iter()
        .filter_map(|item| item["canteen_id"].as_i64())
        .collect();
    assert!(
        canteen_ids.contains(&(fixtures.canteen_id as i64)),
        "results should include original canteen items"
    );
    assert!(
        canteen_ids.contains(&(canteen2_id as i64)),
        "results should include second canteen items"
    );
}

#[actix_rt::test]
async fn search_by_canteen_invalid_canteen_id() {
    let (app, _fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri("/search/99999/Sandwich")
        .insert_header(auth_header())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    let data = body["data"].as_array().expect("data array");
    assert!(
        data.is_empty(),
        "invalid canteen id should return empty results"
    );
}

#[actix_rt::test]
async fn search_by_canteen_unauthenticated() {
    let (app, fixtures, _db_url) = common::setup_api_app().await;

    let req = test::TestRequest::get()
        .uri(&format!("/search/{}/Sandwich", fixtures.canteen_id))
        .to_request();
    common::assert_unauthenticated(&app, req).await;
}
