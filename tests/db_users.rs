mod common;

use proj_xs::db::{RepositoryError, UserOperations};

#[actix_rt::test]
async fn upsert_firebase_user_insert_and_update() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let user_ops = UserOperations::new(pool.clone()).await;

    // Insert a new user
    let user = user_ops
        .upsert_firebase_user(
            "firebase-uid-test".to_string(),
            Some("test@example.com".to_string()),
            Some("Test User".to_string()),
            None,
            true,
        )
        .expect("upsert insert should succeed");

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, "Test User");

    // Upsert again with different email - should update
    let updated_user = user_ops
        .upsert_firebase_user(
            "firebase-uid-test".to_string(),
            Some("updated@example.com".to_string()),
            Some("Updated User".to_string()),
            None,
            true,
        )
        .expect("upsert update should succeed");

    assert_eq!(updated_user.email, "updated@example.com");
    assert_eq!(updated_user.name, "Updated User");
    // Same user_id because same firebase_uid
    assert_eq!(updated_user.user_id, user.user_id);
}

#[actix_rt::test]
async fn upsert_firebase_user_missing_email_errors() {
    let (pool, _fixtures) = common::setup_pool_with_fixtures();
    let user_ops = UserOperations::new(pool.clone()).await;

    let result = user_ops.upsert_firebase_user(
        "firebase-uid-no-email".to_string(),
        None, // no email
        Some("Some User".to_string()),
        None,
        false,
    );

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        RepositoryError::ValidationError(_)
    ));
}
