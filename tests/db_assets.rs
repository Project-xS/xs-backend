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
    // The SDK treats 403 as 404 (garage S3 compatibility).
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

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;
    menu_ops
        .upload_menu_item_pic(&item_id)
        .await
        .expect("upload_menu_item_pic");

    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let key = common::menu_item_pic_key(conn.connection(), item_id).expect("pic_key");
    mock_s3.mock_object_exists(&format!("items/{key}")).await;

    let rows = menu_ops
        .set_menu_item_pic(&item_id)
        .await
        .expect("set_menu_item_pic");
    assert_eq!(rows, 1, "should update exactly one row");

    use diesel::prelude::*;
    use proj_xs::db::schema::menu_items::dsl as mi;
    let etag: Option<String> = mi::menu_items
        .filter(mi::item_id.eq(item_id))
        .select(mi::pic_etag)
        .first(conn.connection())
        .expect("fetch pic_etag");
    assert!(
        etag.is_some(),
        "pic_etag should be set after set_menu_item_pic"
    );
}

#[actix_rt::test]
async fn menu_ops_set_menu_item_pic_not_found_key() {
    let mock_s3 = common::start_mock_s3().await;
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let item_id = fixtures.menu_item_ids[0];

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;
    menu_ops
        .upload_menu_item_pic(&item_id)
        .await
        .expect("upload_menu_item_pic");

    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let key = common::menu_item_pic_key(conn.connection(), item_id).expect("pic_key");
    mock_s3.mock_object_not_found(&format!("items/{key}")).await;

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

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let canteen_ops = CanteenOperations::new(pool.clone(), asset_ops).await;
    canteen_ops
        .upload_canteen_pic(&cid)
        .await
        .expect("upload_canteen_pic");

    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let key = common::canteen_pic_key(conn.connection(), cid).expect("pic_key");
    mock_s3.mock_object_exists(&format!("canteens/{key}")).await;

    let rows = canteen_ops
        .set_canteen_pic(&cid)
        .await
        .expect("set_canteen_pic");
    assert_eq!(rows, 1, "should update exactly one row");

    use diesel::prelude::*;
    use proj_xs::db::schema::canteens::dsl as c;
    let etag: Option<String> = c::canteens
        .filter(c::canteen_id.eq(cid))
        .select(c::pic_etag)
        .first(conn.connection())
        .expect("fetch pic_etag");
    assert!(
        etag.is_some(),
        "pic_etag should be set after set_canteen_pic"
    );
}

#[actix_rt::test]
async fn canteen_ops_set_canteen_pic_not_found_key() {
    let mock_s3 = common::start_mock_s3().await;
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let cid = fixtures.canteen_id;

    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let canteen_ops = CanteenOperations::new(pool.clone(), asset_ops).await;
    canteen_ops
        .upload_canteen_pic(&cid)
        .await
        .expect("upload_canteen_pic");

    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");
    let key = common::canteen_pic_key(conn.connection(), cid).expect("pic_key");
    mock_s3
        .mock_object_not_found(&format!("canteens/{key}"))
        .await;

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
