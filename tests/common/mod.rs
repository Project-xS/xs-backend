//! Test conventions:
//! - Use testcontainers for Postgres when `DATABASE_URL` is not set.
//! - Use dummy S3/AWS env vars via `proj_xs::test_utils::init_test_env`.
//! - Seed fixtures through `proj_xs::test_utils` and keep `has_pic = false`.

use std::env;
use std::sync::OnceLock;

use actix_web::dev::{Service, ServiceRequest, ServiceResponse};
use actix_web::{test, web, App, Error};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use proj_xs::auth::{AdminJwtConfig, AuthLayer, FirebaseAuthConfig, JwksCache};
use proj_xs::test_utils::{
    build_test_pool, init_test_env, reset_db, seed_basic_fixtures, TestFixtures,
};
use proj_xs::{api, AppState};
use testcontainers::clients::Cli;
use testcontainers::images::generic::GenericImage;
use testcontainers::Container;

pub struct TestDb {
    pub database_url: String,
    _container: Option<Container<'static, GenericImage>>,
}

static TEST_DB: OnceLock<TestDb> = OnceLock::new();

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
    impl Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
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

    let app = test::init_service(
        App::new()
            .configure(|cfg| api::configure(cfg, &state, qr_cfg))
            .wrap(AuthLayer::new(
                fb_cfg.clone(),
                admin_cfg.clone(),
                jwks_cache.clone(),
                state.user_ops.clone(),
            ))
            .app_data(web::Data::new(fb_cfg))
            .app_data(web::Data::new(jwks_cache))
            .app_data(web::Data::new(state.user_ops.clone()))
            .app_data(web::Data::new(admin_cfg))
            .app_data(web::JsonConfig::default().error_handler(api::default_error_handler)),
    )
    .await;

    (app, fixtures, db.database_url.clone())
}
