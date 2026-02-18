//! Test conventions:
//! - Use testcontainers for Postgres when `DATABASE_URL` is not set.
//! - Use dummy S3/AWS env vars via `proj_xs::test_utils::init_test_env`.
//! - Seed fixtures through `proj_xs::test_utils` and keep `has_pic = false`.

use std::env;
use std::sync::OnceLock;

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use proj_xs::test_utils::{
    build_test_pool, init_test_env, reset_db, seed_basic_fixtures, TestFixtures,
};
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
