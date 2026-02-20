use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Generate an opaque, HMAC-signed QR token for an order.
///
/// Token format (before base64): `order_id|user_id|timestamp|hex(hmac)`
///
/// The token is base64-encoded so scanning it manually reveals only a random-looking string.
pub fn generate_qr_token(order_id: i32, user_id: i32, secret: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let payload = format!("{}|{}|{}", order_id, user_id, timestamp);
    let signature = sign_payload(&payload, secret);

    let token_raw = format!("{}|{}", payload, signature);
    URL_SAFE_NO_PAD.encode(token_raw.as_bytes())
}

/// Verify a QR token and extract (order_id, user_id, timestamp).
///
/// Returns an error if the token is malformed, the HMAC doesn't match,
/// or the token is older than `max_age_secs`.
pub fn verify_qr_token(token: &str, secret: &str, max_age_secs: u64) -> Result<(i32, i32), String> {
    let decoded_bytes = URL_SAFE_NO_PAD
        .decode(token.trim())
        .map_err(|_| "Invalid token encoding".to_string())?;

    let decoded =
        String::from_utf8(decoded_bytes).map_err(|_| "Invalid token content".to_string())?;

    let parts: Vec<&str> = decoded.splitn(4, '|').collect();
    if parts.len() != 4 {
        return Err("Malformed token".to_string());
    }

    let order_id: i32 = parts[0]
        .parse()
        .map_err(|_| "Invalid token data".to_string())?;
    let user_id: i32 = parts[1]
        .parse()
        .map_err(|_| "Invalid token data".to_string())?;
    let timestamp: u64 = parts[2]
        .parse()
        .map_err(|_| "Invalid token data".to_string())?;
    let provided_signature = parts[3];

    // Reconstruct payload and verify HMAC
    let payload = format!("{}|{}|{}", order_id, user_id, timestamp);
    let expected_signature = sign_payload(&payload, secret);

    // Constant-time comparison via HMAC verification
    if !constant_time_eq(provided_signature.as_bytes(), expected_signature.as_bytes()) {
        return Err("Invalid token signature".to_string());
    }

    // Check token age
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now.saturating_sub(timestamp) > max_age_secs {
        return Err("Token has expired".to_string());
    }

    Ok((order_id, user_id))
}

/// HMAC-SHA256 sign a payload string, returning the hex-encoded signature.
fn sign_payload(payload: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time byte comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_expired_token() {
        let secret = "test-secret";
        // Construct a token with timestamp = 1 (Unix epoch start) which is definitely expired
        let payload = "1|1|1";
        let signature = sign_payload(payload, secret);
        let token_raw = format!("{}|{}", payload, signature);
        let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_raw.as_bytes());
        let result = verify_qr_token(&token, secret, 60);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Token has expired");
    }

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"hellx"));
        assert!(!constant_time_eq(b"hello", b"hello!"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(constant_time_eq(b"", b""));
    }
}
