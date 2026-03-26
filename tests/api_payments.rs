mod common;

use actix_web::http::header;
use actix_web::http::StatusCode;
use actix_web::test;
use common::auth_header;
use proj_xs::db::DbConnection;
use proj_xs::test_utils::build_test_pool;
use serde_json::Value;
use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn configure_phonepe_mock_env(base_url: &str) {
    std::env::set_var("PHONEPE_AUTH_BASE_URL", base_url);
    std::env::set_var("PHONEPE_PG_BASE_URL", base_url);
}

fn webhook_hash_header_value() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"test-phonepe-webhook-user:test-phonepe-webhook-password");
    hex::encode(hasher.finalize())
}

async fn mock_phonepe_oauth(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "oauth_token_test",
            "expires_at": 4_102_444_800i64
        })))
        .mount(server)
        .await;
}

async fn create_hold(
    app: &impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<actix_web::body::BoxBody>,
        Error = actix_web::Error,
    >,
    user_id: i32,
    item_id: i32,
) -> i64 {
    let hold_req = test::TestRequest::post()
        .uri(&format!("/orders/hold?as=user-{}", user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "deliver_at": null,
            "item_ids": [item_id]
        }))
        .to_request();
    let hold_resp = test::call_service(app, hold_req).await;
    assert_eq!(hold_resp.status(), StatusCode::OK);
    let hold_body: Value = test::read_body_json(hold_resp).await;
    hold_body["hold_id"].as_i64().expect("hold_id")
}

#[actix_rt::test]
async fn payments_initiate_success_and_reuse_existing_mapping() {
    let mock_server = MockServer::start().await;
    configure_phonepe_mock_env(&mock_server.uri());
    mock_phonepe_oauth(&mock_server).await;

    Mock::given(method("POST"))
        .and(path("/checkout/v2/sdk/order"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "orderId": "O12345678",
            "token": "sdk_token_abc",
            "merchantId": "MERCHANT_ID_TEST"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let (app, fixtures, _db_url) = common::setup_api_app().await;
    let hold_id = create_hold(&app, fixtures.user_id, fixtures.menu_item_ids[0]).await;

    let initiate_req = test::TestRequest::post()
        .uri(&format!("/payments/initiate?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "hold_id": hold_id,
            "amount": 120
        }))
        .to_request();
    let initiate_resp = test::call_service(&app, initiate_req).await;
    assert_eq!(initiate_resp.status(), StatusCode::OK);
    let first_body: Value = test::read_body_json(initiate_resp).await;
    assert_eq!(first_body["status"], "ok");
    assert_eq!(first_body["order_id"], "O12345678");
    assert_eq!(first_body["token"], "sdk_token_abc");

    let initiate_req_2 = test::TestRequest::post()
        .uri(&format!("/payments/initiate?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "hold_id": hold_id,
            "amount": 120
        }))
        .to_request();
    let initiate_resp_2 = test::call_service(&app, initiate_req_2).await;
    assert_eq!(initiate_resp_2.status(), StatusCode::OK);
    let second_body: Value = test::read_body_json(initiate_resp_2).await;
    assert_eq!(second_body["status"], "ok");
    assert_eq!(
        second_body["merchant_order_id"],
        first_body["merchant_order_id"]
    );
    assert_eq!(second_body["order_id"], first_body["order_id"]);
    assert_eq!(second_body["token"], first_body["token"]);
}

#[actix_rt::test]
async fn payments_verify_completed_confirms_hold_and_creates_order() {
    let mock_server = MockServer::start().await;
    configure_phonepe_mock_env(&mock_server.uri());
    mock_phonepe_oauth(&mock_server).await;

    Mock::given(method("POST"))
        .and(path("/checkout/v2/sdk/order"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "orderId": "O998877",
            "token": "sdk_token_xyz",
            "merchantId": "MERCHANT_ID_TEST"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/checkout/v2/order/.+/status$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "state": "COMPLETED"
        })))
        .mount(&mock_server)
        .await;

    let (app, fixtures, db_url) = common::setup_api_app().await;
    let hold_id = create_hold(&app, fixtures.user_id, fixtures.menu_item_ids[0]).await;

    let initiate_req = test::TestRequest::post()
        .uri(&format!("/payments/initiate?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "hold_id": hold_id,
            "amount": 120
        }))
        .to_request();
    let initiate_resp = test::call_service(&app, initiate_req).await;
    assert_eq!(initiate_resp.status(), StatusCode::OK);
    let initiate_body: Value = test::read_body_json(initiate_resp).await;
    let merchant_order_id = initiate_body["merchant_order_id"]
        .as_str()
        .expect("merchant_order_id");

    let verify_req = test::TestRequest::post()
        .uri(&format!(
            "/payments/verify/{}?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "merchant_order_id": merchant_order_id
        }))
        .to_request();
    let verify_resp = test::call_service(&app, verify_req).await;
    assert_eq!(verify_resp.status(), StatusCode::OK);
    let verify_body: Value = test::read_body_json(verify_resp).await;
    assert_eq!(verify_body["status"], "ok");
    assert_eq!(verify_body["payment_state"], "COMPLETED");
    assert!(verify_body["order_id"].is_number());

    let pool = build_test_pool(&db_url);
    let mut conn = DbConnection::new(&pool).expect("db conn");
    assert_eq!(common::held_orders_count(conn.connection()), 0);
    assert_eq!(common::active_orders_count(conn.connection()), 1);
}

#[actix_rt::test]
async fn payments_verify_pending_and_failed_paths() {
    let mock_server = MockServer::start().await;
    configure_phonepe_mock_env(&mock_server.uri());
    mock_phonepe_oauth(&mock_server).await;

    Mock::given(method("POST"))
        .and(path("/checkout/v2/sdk/order"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "orderId": "OPENDING1",
            "token": "sdk_token_pending",
            "merchantId": "MERCHANT_ID_TEST"
        })))
        .mount(&mock_server)
        .await;

    // First verify => pending, second verify => failed
    Mock::given(method("GET"))
        .and(path_regex(r"^/checkout/v2/order/.+/status$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "state": "PENDING"
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/checkout/v2/order/.+/status$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "state": "FAILED"
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let (app, fixtures, db_url) = common::setup_api_app().await;
    let hold_id = create_hold(&app, fixtures.user_id, fixtures.menu_item_ids[0]).await;

    let initiate_req = test::TestRequest::post()
        .uri(&format!("/payments/initiate?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "hold_id": hold_id,
            "amount": 120
        }))
        .to_request();
    let initiate_resp = test::call_service(&app, initiate_req).await;
    assert_eq!(initiate_resp.status(), StatusCode::OK);
    let initiate_body: Value = test::read_body_json(initiate_resp).await;
    let merchant_order_id = initiate_body["merchant_order_id"]
        .as_str()
        .expect("merchant_order_id");

    let verify_pending_req = test::TestRequest::post()
        .uri(&format!(
            "/payments/verify/{}?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "merchant_order_id": merchant_order_id
        }))
        .to_request();
    let verify_pending_resp = test::call_service(&app, verify_pending_req).await;
    assert_eq!(verify_pending_resp.status(), StatusCode::CONFLICT);
    let pending_body: Value = test::read_body_json(verify_pending_resp).await;
    assert_eq!(pending_body["payment_state"], "PENDING");

    let verify_failed_req = test::TestRequest::post()
        .uri(&format!(
            "/payments/verify/{}?as=user-{}",
            hold_id, fixtures.user_id
        ))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "merchant_order_id": merchant_order_id
        }))
        .to_request();
    let verify_failed_resp = test::call_service(&app, verify_failed_req).await;
    assert_eq!(verify_failed_resp.status(), StatusCode::CONFLICT);
    let failed_body: Value = test::read_body_json(verify_failed_resp).await;
    assert_eq!(failed_body["payment_state"], "FAILED");

    let pool = build_test_pool(&db_url);
    let mut conn = DbConnection::new(&pool).expect("db conn");
    assert_eq!(common::held_orders_count(conn.connection()), 0);
}

#[actix_rt::test]
async fn payments_webhook_auth_and_idempotency() {
    let mock_server = MockServer::start().await;
    configure_phonepe_mock_env(&mock_server.uri());
    mock_phonepe_oauth(&mock_server).await;

    Mock::given(method("POST"))
        .and(path("/checkout/v2/sdk/order"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "orderId": "OWEBHOOK1",
            "token": "sdk_token_webhook",
            "merchantId": "MERCHANT_ID_TEST"
        })))
        .mount(&mock_server)
        .await;

    let (app, fixtures, db_url) = common::setup_api_app().await;
    let hold_id = create_hold(&app, fixtures.user_id, fixtures.menu_item_ids[0]).await;

    let initiate_req = test::TestRequest::post()
        .uri(&format!("/payments/initiate?as=user-{}", fixtures.user_id))
        .insert_header(auth_header())
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .set_json(&serde_json::json!({
            "hold_id": hold_id,
            "amount": 120
        }))
        .to_request();
    let initiate_resp = test::call_service(&app, initiate_req).await;
    assert_eq!(initiate_resp.status(), StatusCode::OK);
    let initiate_body: Value = test::read_body_json(initiate_resp).await;
    let merchant_order_id = initiate_body["merchant_order_id"]
        .as_str()
        .expect("merchant_order_id")
        .to_string();

    let bad_webhook_req = test::TestRequest::post()
        .uri("/payments/webhook")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, "invalid"))
        .set_json(&serde_json::json!({
            "event": "checkout.order.completed",
            "payload": {
                "merchantOrderId": merchant_order_id,
                "state": "COMPLETED"
            }
        }))
        .to_request();
    let bad_webhook_resp = test::call_service(&app, bad_webhook_req).await;
    assert_eq!(bad_webhook_resp.status(), StatusCode::EXPECTATION_FAILED);

    let good_auth = webhook_hash_header_value();
    let validation_req = test::TestRequest::post()
        .uri("/payments/webhook")
        .insert_header((
            header::AUTHORIZATION,
            format!("SHA256({})", good_auth.clone()),
        ))
        .to_request();
    let validation_resp = test::call_service(&app, validation_req).await;
    assert_eq!(validation_resp.status(), StatusCode::BAD_REQUEST);

    let webhook_req = test::TestRequest::post()
        .uri("/payments/webhook")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((
            header::AUTHORIZATION,
            format!("SHA256({})", good_auth.clone()),
        ))
        .set_json(&serde_json::json!({
            "event": "checkout.order.completed",
            "payload": {
                "merchantOrderId": merchant_order_id,
                "state": "COMPLETED"
            }
        }))
        .to_request();
    let webhook_resp = test::call_service(&app, webhook_req).await;
    assert_eq!(webhook_resp.status(), StatusCode::OK);

    // Idempotent replay
    let webhook_req_replay = test::TestRequest::post()
        .uri("/payments/webhook")
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .insert_header((header::AUTHORIZATION, good_auth))
        .set_json(&serde_json::json!({
            "event": "checkout.order.completed",
            "payload": {
                "merchantOrderId": merchant_order_id,
                "state": "COMPLETED"
            }
        }))
        .to_request();
    let webhook_replay_resp = test::call_service(&app, webhook_req_replay).await;
    assert_eq!(webhook_replay_resp.status(), StatusCode::OK);

    let pool = build_test_pool(&db_url);
    let mut conn = DbConnection::new(&pool).expect("db conn");
    assert_eq!(common::active_orders_count(conn.connection()), 1);
}
