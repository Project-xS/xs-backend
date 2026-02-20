#![allow(dead_code)]
//! Test conventions:
//! - Use testcontainers for Postgres when `DATABASE_URL` is not set.
//! - Use dummy S3/AWS env vars via `proj_xs::test_utils::init_test_env`.
//! - Seed fixtures through `proj_xs::test_utils` and keep `has_pic = false`.

use std::env;
use std::sync::OnceLock;

use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::{test, web, App, Error};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use proj_xs::auth::{AdminJwtConfig, AuthLayer, FirebaseAuthConfig, JwksCache};
use proj_xs::test_utils::{
    build_test_pool, init_test_env, reset_db, seed_basic_fixtures, TestFixtures,
};
use proj_xs::{api, AppState};
use testcontainers::clients::Cli;
use testcontainers::Container;
use testcontainers::GenericImage;
use utoipa_actix_web::AppExt;

pub struct TestDb {
    pub database_url: String,
    _container: Option<Container<'static, GenericImage>>,
}

static TEST_DB: OnceLock<TestDb> = OnceLock::new();

/// Calls `try_call_service` and asserts the response is 401 Unauthorized.
/// Use for endpoints that should reject unauthenticated requests.
pub async fn assert_unauthenticated<S>(app: &S, req: Request)
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = Error>,
{
    let status = match test::try_call_service(app, req).await {
        Ok(r) => r.status(),
        Err(e) => e.as_response_error().status_code(),
    };
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

pub fn auth_header() -> (header::HeaderName, String) {
    let token = std::env::var("DEV_BYPASS_TOKEN").expect("DEV_BYPASS_TOKEN");
    (header::AUTHORIZATION, format!("Bearer {}", token))
}

pub fn active_orders_count(conn: &mut PgConnection) -> i64 {
    proj_xs::db::schema::active_orders::table
        .count()
        .get_result(conn)
        .expect("count active_orders")
}

pub fn active_order_items_count(conn: &mut PgConnection) -> i64 {
    proj_xs::db::schema::active_order_items::table
        .count()
        .get_result(conn)
        .expect("count active_order_items")
}

pub fn held_orders_count(conn: &mut PgConnection) -> i64 {
    proj_xs::db::schema::held_orders::table
        .count()
        .get_result(conn)
        .expect("count held_orders")
}

pub fn held_order_items_count(conn: &mut PgConnection) -> i64 {
    proj_xs::db::schema::held_order_items::table
        .count()
        .get_result(conn)
        .expect("count held_order_items")
}

pub fn menu_item_state(conn: &mut PgConnection, item_id_val: i32) -> (i32, bool) {
    use proj_xs::db::schema::menu_items::dsl::*;
    menu_items
        .filter(item_id.eq(item_id_val))
        .select((stock, is_available))
        .first::<(i32, bool)>(conn)
        .expect("menu item state")
}

pub fn setup_test_db() -> &'static TestDb {
    TEST_DB.get_or_init(|| {
        if let Ok(url) = env::var("DATABASE_URL") {
            return TestDb {
                database_url: url,
                _container: None,
            };
        }

        let docker = Box::leak(Box::new(Cli::default()));
        let image = GenericImage::new("postgres", "16-alpine")
            .with_env_var("POSTGRES_USER", "postgres")
            .with_env_var("POSTGRES_PASSWORD", "postgres")
            .with_env_var("POSTGRES_DB", "proj_xs_test")
            .with_exposed_port(5432);

        let container = docker.run(image);
        let port = container.get_host_port_ipv4(5432);
        let database_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/proj_xs_test");

        TestDb {
            database_url,
            _container: Some(container),
        }
    })
}

pub fn setup_pool() -> Pool<ConnectionManager<PgConnection>> {
    init_test_env();
    let db = setup_test_db();
    let pool = build_test_pool(&db.database_url);
    reset_db(&pool).expect("reset db");
    pool
}

pub fn setup_pool_with_fixtures() -> (Pool<ConnectionManager<PgConnection>>, TestFixtures) {
    let pool = setup_pool();
    let fixtures = seed_basic_fixtures(&pool).expect("seed fixtures");
    (pool, fixtures)
}

pub async fn setup_api_app() -> (
    impl Service<Request, Response = ServiceResponse<BoxBody>, Error = Error>,
    TestFixtures,
    String,
) {
    init_test_env();
    let db = setup_test_db();
    let pool = build_test_pool(&db.database_url);
    reset_db(&pool).expect("reset db");
    let fixtures = seed_basic_fixtures(&pool).expect("seed fixtures");

    let state = AppState::new(&db.database_url).await;

    let fb_cfg = FirebaseAuthConfig::from_env();
    let admin_cfg = AdminJwtConfig::from_env();
    let jwks_cache = JwksCache::new(fb_cfg.jwks_url.clone(), fb_cfg.cache_ttl_secs);

    let qr_secret =
        std::env::var("DELIVER_QR_HASH_SECRET").expect("DELIVER_QR_HASH_SECRET must be set");
    let qr_max_age: u64 = std::env::var("QR_TOKEN_MAX_AGE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(86400);

    let qr_cfg = api::common::qr::QrConfig {
        secret: qr_secret,
        max_age_secs: qr_max_age,
    };

    let app = App::new()
        .into_utoipa_app()
        .configure(|cfg| api::configure(cfg, &state, qr_cfg))
        .map(|app| {
            app.wrap(AuthLayer::new(
                fb_cfg.clone(),
                admin_cfg.clone(),
                jwks_cache.clone(),
                state.user_ops.clone(),
            ))
            .app_data(web::Data::new(fb_cfg))
            .app_data(web::Data::new(jwks_cache))
            .app_data(web::Data::new(state.user_ops.clone()))
            .app_data(web::Data::new(admin_cfg))
            .app_data(web::JsonConfig::default().error_handler(api::default_error_handler))
        })
        .into_app();

    let app = test::init_service(app).await;

    (app, fixtures, db.database_url.clone())
}

// ---------------------------------------------------------------------------
// Mock S3
// ---------------------------------------------------------------------------

/// RAII guard: runs a wiremock S3 server, overrides `S3_ENDPOINT`, restores on drop.
pub struct MockS3Guard {
    pub server: wiremock::MockServer,
    previous_endpoint: Option<String>,
}

impl Drop for MockS3Guard {
    fn drop(&mut self) {
        match &self.previous_endpoint {
            Some(v) => std::env::set_var("S3_ENDPOINT", v),
            None => std::env::remove_var("S3_ENDPOINT"),
        }
    }
}

impl MockS3Guard {
    pub fn uri(&self) -> String {
        self.server.uri()
    }

    /// Mock `GET /{bucket}/{key_suffix}` → 200 with an ETag.
    pub async fn mock_object_exists(&self, key_suffix: &str) {
        let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "test-bucket".to_string());
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(format!(
                "/{}/{}",
                bucket, key_suffix
            )))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .insert_header("etag", "\"test-etag-abc123\"")
                    .set_body_bytes(b"fake-image-data"),
            )
            .mount(&self.server)
            .await;
    }

    /// Mock `GET /{bucket}/{key_suffix}` → 404.
    pub async fn mock_object_not_found(&self, key_suffix: &str) {
        let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "test-bucket".to_string());
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(format!(
                "/{}/{}",
                bucket, key_suffix
            )))
            .respond_with(
                wiremock::ResponseTemplate::new(404).set_body_string(
                    r#"<?xml version="1.0" encoding="UTF-8"?><Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message></Error>"#,
                ),
            )
            .mount(&self.server)
            .await;
    }
}

/// Start a mock S3 server. Must be called **before** `setup_api_app()` / `AssetOperations::new()`.
pub async fn start_mock_s3() -> MockS3Guard {
    // Ensure all other test env vars are initialised first.
    init_test_env();
    let previous_endpoint = std::env::var("S3_ENDPOINT").ok();
    let server = wiremock::MockServer::start().await;
    std::env::set_var("S3_ENDPOINT", server.uri());
    MockS3Guard {
        server,
        previous_endpoint,
    }
}
