mod common;

use proj_xs::db::{AssetOperations, DbConnection, MenuOperations, RepositoryError};
use proj_xs::models::admin::{NewMenuItem, UpdateMenuItem};

#[actix_rt::test]
async fn add_menu_item_success() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    let new_item = NewMenuItem {
        canteen_id: fixtures.canteen_id,
        name: "Paneer Tikka".to_string(),
        is_veg: true,
        price: 150,
        stock: 20,
        is_available: true,
        description: Some("Grilled paneer".to_string()),
        has_pic: false,
    };

    let result = menu_ops.add_menu_item(new_item);
    assert!(result.is_ok(), "add_menu_item should succeed: {:?}", result);
    let item = result.unwrap();
    assert_eq!(item.name, "Paneer Tikka");
    assert_eq!(item.price, 150);
    assert_eq!(item.canteen_id, fixtures.canteen_id);
    assert!(item.is_veg);
}

#[actix_rt::test]
async fn add_menu_item_rejects_zero_price() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    let new_item = NewMenuItem {
        canteen_id: fixtures.canteen_id,
        name: "Free Item".to_string(),
        is_veg: true,
        price: 0,
        stock: 10,
        is_available: true,
        description: None,
        has_pic: false,
    };

    let result = menu_ops.add_menu_item(new_item);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        RepositoryError::ValidationError(_)
    ));
}

#[actix_rt::test]
async fn add_menu_item_rejects_empty_name() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    let new_item = NewMenuItem {
        canteen_id: fixtures.canteen_id,
        name: "   ".to_string(),
        is_veg: true,
        price: 100,
        stock: 10,
        is_available: true,
        description: None,
        has_pic: false,
    };

    let result = menu_ops.add_menu_item(new_item);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        RepositoryError::ValidationError(_)
    ));
}

#[actix_rt::test]
async fn update_menu_item_partial_name_only() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    let item_id = fixtures.menu_item_ids[0];
    let update = UpdateMenuItem {
        name: Some("Updated Name".to_string()),
        is_veg: None,
        price: None,
        stock: None,
        is_available: None,
        description: None,
    };

    let result = menu_ops.update_menu_item(item_id, update);
    assert!(result.is_ok(), "update should succeed: {:?}", result);
    let updated = result.unwrap();
    assert_eq!(updated.name, "Updated Name");
    // Other fields should be unchanged
    assert_eq!(updated.price, 120); // Veg Sandwich original price
    assert_eq!(updated.stock, 10); // original stock
}

#[actix_rt::test]
async fn update_menu_item_not_found() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    let update = UpdateMenuItem {
        name: Some("Ghost".to_string()),
        is_veg: None,
        price: None,
        stock: None,
        is_available: None,
        description: None,
    };

    let result = menu_ops.update_menu_item(99999, update);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RepositoryError::NotFound(_)));
}

#[actix_rt::test]
async fn remove_menu_item_success() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    let target_id = fixtures.menu_item_ids[0];
    let result = menu_ops.remove_menu_item(target_id);
    assert!(result.is_ok(), "remove should succeed: {:?}", result);
    assert_eq!(result.unwrap().item_id, target_id);

    // Verify it's gone
    use diesel::prelude::*;
    use proj_xs::db::schema::menu_items::dsl as mi_dsl;
    let mut conn = DbConnection::new(&pool).expect("db connection");
    let count: i64 = mi_dsl::menu_items
        .filter(mi_dsl::item_id.eq(target_id))
        .count()
        .get_result(conn.connection())
        .expect("count");
    assert_eq!(count, 0, "item should be deleted");
}

#[actix_rt::test]
async fn remove_menu_item_not_found() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    let result = menu_ops.remove_menu_item(99999);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RepositoryError::NotFound(_)));
}

#[actix_rt::test]
async fn get_menu_item_by_id_and_not_found() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let asset_ops = AssetOperations::new().await.expect("AssetOperations::new");
    let menu_ops = MenuOperations::new(pool.clone(), asset_ops).await;

    // Existing item
    let target_id = fixtures.menu_item_ids[0];
    let result = menu_ops.get_menu_item(target_id).await;
    assert!(result.is_ok(), "get_menu_item should succeed: {:?}", result);
    let item = result.unwrap();
    assert_eq!(item.item_id, target_id);

    // Non-existent item
    let result = menu_ops.get_menu_item(99999).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), RepositoryError::NotFound(_)));
}
