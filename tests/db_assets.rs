mod common;

use proj_xs::db::{AssetOperations, CanteenOperations, MenuOperations, RepositoryError, S3Error};

// ── AssetOperations (raw S3 layer) ──────────────────────────────────────────

#[actix_rt::test]
async fn asset_ops_get_object_etag_success() {
    let mock_s3 = common::start_mock_s3().await;
    mock_s3.mock_object_exists("items/1").await;

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let etag = asset_ops
        .get_object_etag("items/1")
        .await
        .expect("get_object_etag");
    assert!(etag.is_some(), "etag should be present for existing object");
}

#[actix_rt::test]
async fn asset_ops_get_object_etag_not_found_on_404() {
    let mock_s3 = common::start_mock_s3().await;
    mock_s3.mock_object_not_found("items/999").await;

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let err = asset_ops
        .get_object_etag("items/999")
        .await
        .expect_err("should return S3Error::NotFound");
    assert!(
        matches!(err, S3Error::NotFound(_)),
        "expected NotFound, got {:?}",
        err
    );
}

#[actix_rt::test]
async fn asset_ops_get_object_etag_not_found_on_403() {
    // The SDK treats 403 the same as 404 (garage compatibility hack in canteen.rs).
    let mock_s3 = common::start_mock_s3().await;
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "test-bucket".to_string());
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(format!(
            "/{}/items/403test",
            bucket
        )))
        .respond_with(wiremock::ResponseTemplate::new(403))
        .mount(&mock_s3.server)
        .await;

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let err = asset_ops
        .get_object_etag("items/403test")
        .await
        .expect_err("should return S3Error::NotFound on 403");
    assert!(
        matches!(err, S3Error::NotFound(_)),
        "expected NotFound, got {:?}",
        err
    );
}

#[actix_rt::test]
async fn asset_ops_get_object_presign_success() {
    let mock_s3 = common::start_mock_s3().await;
    mock_s3.mock_object_exists("items/42").await;

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let url = asset_ops
        .get_object_presign("items/42")
        .await
        .expect("get_object_presign");
    assert!(
        url.starts_with("http"),
        "presigned URL should be http(s): {}",
        url
    );
}

// ── MenuOperations.set_menu_item_pic (DB + S3) ───────────────────────────────

#[actix_rt::test]
async fn menu_ops_set_menu_item_pic_success() {
    let mock_s3 = common::start_mock_s3().await;
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let item_id = fixtures.menu_item_ids[0];

    mock_s3
        .mock_object_exists(&format!("items/{}", item_id))
        .await;

    let menu_ops = MenuOperations::new(pool.clone()).await;
    let rows = menu_ops
        .set_menu_item_pic(&item_id)
        .await
        .expect("set_menu_item_pic");
    assert_eq!(rows, 1, "should update exactly one row");

    // Verify has_pic was set in the DB
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    use diesel::prelude::*;
    use proj_xs::db::schema::menu_items::dsl as mi;
    let pic_flag: bool = mi::menu_items
        .filter(mi::item_id.eq(item_id))
        .select(mi::has_pic)
        .first(conn.connection())
        .expect("fetch has_pic");
    assert!(pic_flag, "has_pic should be true after set_menu_item_pic");
}

#[actix_rt::test]
async fn menu_ops_set_menu_item_pic_not_found_key() {
    let mock_s3 = common::start_mock_s3().await;
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let item_id = fixtures.menu_item_ids[0];

    mock_s3
        .mock_object_not_found(&format!("items/{}", item_id))
        .await;

    let menu_ops = MenuOperations::new(pool.clone()).await;
    let err = menu_ops
        .set_menu_item_pic(&item_id)
        .await
        .expect_err("should fail when object not found");
    assert!(
        matches!(err, RepositoryError::AssetError(_)),
        "expected AssetError (from S3Error::NotFound), got {:?}",
        err
    );
}

// ── CanteenOperations.set_canteen_pic (DB + S3) ──────────────────────────────

#[actix_rt::test]
async fn canteen_ops_set_canteen_pic_success() {
    let mock_s3 = common::start_mock_s3().await;
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let cid = fixtures.canteen_id;

    // set_canteen_pic uses key "canteens/{canteen_id}" after the bug fix
    mock_s3
        .mock_object_exists(&format!("canteens/{}", cid))
        .await;

    let canteen_ops = CanteenOperations::new(pool.clone()).await;
    let rows = canteen_ops
        .set_canteen_pic(&cid)
        .await
        .expect("set_canteen_pic");
    assert_eq!(rows, 1, "should update exactly one row");

    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    use diesel::prelude::*;
    use proj_xs::db::schema::canteens::dsl as c;
    let pic_flag: bool = c::canteens
        .filter(c::canteen_id.eq(cid))
        .select(c::has_pic)
        .first(conn.connection())
        .expect("fetch has_pic");
    assert!(pic_flag, "has_pic should be true after set_canteen_pic");
}

#[actix_rt::test]
async fn canteen_ops_set_canteen_pic_not_found_key() {
    let mock_s3 = common::start_mock_s3().await;
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let cid = fixtures.canteen_id;

    mock_s3
        .mock_object_not_found(&format!("canteens/{}", cid))
        .await;

    let canteen_ops = CanteenOperations::new(pool.clone()).await;
    let err = canteen_ops
        .set_canteen_pic(&cid)
        .await
        .expect_err("should fail when object not found");
    assert!(
        matches!(err, RepositoryError::AssetError(_)),
        "expected AssetError (from S3Error::NotFound), got {:?}",
        err
    );
}
