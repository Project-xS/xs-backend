#[macro_use]
extern crate log;

pub mod api;
pub mod auth;
pub mod db;
pub mod enums;
pub mod models;
pub mod test_utils;
pub mod traits;

use crate::db::{
    establish_connection_pool, run_db_migrations, AssetOperations, CanteenOperations,
    HoldOperations, MenuOperations, OrderOperations, SearchOperations, UserOperations,
};

#[derive(Clone)]
pub struct AppState {
    pub user_ops: UserOperations,
    pub menu_ops: MenuOperations,
    pub canteen_ops: CanteenOperations,
    pub order_ops: OrderOperations,
    pub hold_ops: HoldOperations,
    pub search_ops: SearchOperations,
    pub asset_ops: AssetOperations,
}

impl AppState {
    pub async fn new(url: &str) -> Self {
        let db = establish_connection_pool(url);
        run_db_migrations(db.clone()).expect("Unable to run migrations");

        let hold_ttl_secs: i64 = std::env::var("ORDER_HOLD_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(300); // 5 minutes default

        let asset_ops = AssetOperations::new()
            .await
            .expect("Unable to create asset_ops");

        let user_ops = UserOperations::new(db.clone(), asset_ops.clone()).await;
        let menu_ops = MenuOperations::new(db.clone(), asset_ops.clone()).await;
        let canteen_ops = CanteenOperations::new(db.clone(), asset_ops.clone()).await;
        let order_ops = OrderOperations::new(db.clone()).await;
        let hold_ops = HoldOperations::new(db.clone(), hold_ttl_secs);
        let search_ops = SearchOperations::new(db.clone()).await;
        AppState {
            user_ops,
            menu_ops,
            canteen_ops,
            order_ops,
            hold_ops,
            search_ops,
            asset_ops,
        }
    }
}
