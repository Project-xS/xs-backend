mod common;

use proj_xs::db::SearchOperations;
use proj_xs::test_utils::{insert_canteen, seed_menu_item};

#[actix_rt::test]
async fn search_returns_matching_items() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let search_ops = SearchOperations::new(pool.clone()).await;

    // "Veg Sandwich" is seeded; use full name to ensure pg_trgm similarity match
    let results = search_ops
        .search_menu_items("Veg Sandwich")
        .await
        .expect("search should succeed");

    assert!(
        !results.is_empty(),
        "search for 'Veg Sandwich' should return results"
    );
    let names: Vec<&str> = results.iter().map(|i| i.name.as_str()).collect();
    assert!(
        names.contains(&"Veg Sandwich"),
        "result should contain the seeded Veg Sandwich"
    );
}

#[actix_rt::test]
async fn search_no_match_returns_empty() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let search_ops = SearchOperations::new(pool.clone()).await;

    let results = search_ops
        .search_menu_items("zzzznonexistent")
        .await
        .expect("search should succeed even with no results");

    assert!(
        results.is_empty(),
        "no items should match 'zzzznonexistent'"
    );
}

#[actix_rt::test]
async fn search_by_canteen_filters_correctly() {
    let (pool, fixtures) = common::setup_pool_with_fixtures();
    let mut conn = proj_xs::db::DbConnection::new(&pool).expect("db connection");

    // Create a second canteen with a uniquely-named item
    let other_canteen =
        insert_canteen(conn.connection(), "Other Canteen", "Block Z").expect("insert canteen");
    seed_menu_item(
        conn.connection(),
        other_canteen,
        "Veg Sandwich",
        100,
        10,
        true,
        true,
        None,
    )
    .expect("insert item");

    let search_ops = SearchOperations::new(pool.clone()).await;

    // Search scoped to the original canteen
    let results = search_ops
        .search_menu_items_by_canteen(&fixtures.canteen_id, "Veg Sandwich")
        .await
        .expect("search should succeed");

    // All results should belong to fixtures.canteen_id, not other_canteen
    assert!(
        !results.is_empty(),
        "should find items in the correct canteen"
    );
    for item in &results {
        assert_eq!(
            item.canteen_id, fixtures.canteen_id,
            "all results should belong to the searched canteen"
        );
    }
}
