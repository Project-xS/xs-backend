use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use proj_xs::auth::qr_token::{generate_qr_token, verify_qr_token};

#[test]
fn generate_and_verify_round_trip() {
    let secret = "test-secret";
    let order_id = 42;
    let user_id = 7;
    let token = generate_qr_token(order_id, user_id, secret);
    let result = verify_qr_token(&token, secret, 86400);
    assert!(result.is_ok(), "round trip should succeed: {:?}", result);
    let (got_order_id, got_user_id) = result.unwrap();
    assert_eq!(got_order_id, order_id);
    assert_eq!(got_user_id, user_id);
}

#[test]
fn verify_tampered_token() {
    let secret = "test-secret";
    let token = generate_qr_token(1, 1, secret);
    // Decode, flip a byte, re-encode
    let mut bytes = URL_SAFE_NO_PAD.decode(&token).unwrap();
    bytes[0] ^= 0xFF;
    let tampered = URL_SAFE_NO_PAD.encode(&bytes);
    let result = verify_qr_token(&tampered, secret, 86400);
    assert!(result.is_err(), "tampered token should fail");
}

#[test]
fn verify_malformed_token() {
    // base64-encode a string without 4 pipe-separated parts
    let raw = "only|three|parts";
    let token = URL_SAFE_NO_PAD.encode(raw.as_bytes());
    let result = verify_qr_token(&token, "secret", 86400);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Malformed token");
}

#[test]
fn verify_invalid_base64() {
    let result = verify_qr_token("!!!not-base64!!!", "secret", 86400);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid token encoding");
}

#[test]
fn verify_wrong_secret() {
    let token = generate_qr_token(1, 1, "secret-a");
    let result = verify_qr_token(&token, "secret-b", 86400);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid token signature");
}
