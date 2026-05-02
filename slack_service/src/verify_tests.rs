use super::verify_request;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const SECRET: &str = "test_signing_secret";
const NOW: i64 = 1_000_000;
const BODY: &[u8] = b"token=test&command=%2Ffruit";

fn make_sig(secret: &str, timestamp: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(b"v0:");
    mac.update(timestamp.as_bytes());
    mac.update(b":");
    mac.update(body);
    format!("v0={}", hex::encode(mac.finalize().into_bytes()))
}

#[test]
fn valid_request_passes() {
    let ts = NOW.to_string();
    let sig = make_sig(SECRET, &ts, BODY);
    assert!(verify_request(SECRET, NOW, &ts, BODY, &sig));
}

#[test]
fn wrong_secret_fails() {
    let ts = NOW.to_string();
    let sig = make_sig("wrong_secret", &ts, BODY);
    assert!(!verify_request(SECRET, NOW, &ts, BODY, &sig));
}

#[test]
fn expired_timestamp_fails() {
    let ts = (NOW - 301).to_string();
    let sig = make_sig(SECRET, &ts, BODY);
    assert!(!verify_request(SECRET, NOW, &ts, BODY, &sig));
}

#[test]
fn timestamp_at_boundary_300s_passes() {
    let ts = (NOW - 300).to_string();
    let sig = make_sig(SECRET, &ts, BODY);
    assert!(verify_request(SECRET, NOW, &ts, BODY, &sig));
}

#[test]
fn future_timestamp_beyond_window_fails() {
    // Without .abs(), NOW - (NOW+301) = -301 ≤ 300 would incorrectly pass.
    let ts = (NOW + 301).to_string();
    let sig = make_sig(SECRET, &ts, BODY);
    assert!(!verify_request(SECRET, NOW, &ts, BODY, &sig));
}

#[test]
fn non_numeric_timestamp_fails() {
    let sig = make_sig(SECRET, "not_a_number", BODY);
    assert!(!verify_request(SECRET, NOW, "not_a_number", BODY, &sig));
}

#[test]
fn missing_v0_prefix_fails() {
    let ts = NOW.to_string();
    let valid_sig = make_sig(SECRET, &ts, BODY);
    let without_prefix = &valid_sig["v0=".len()..];
    assert!(!verify_request(SECRET, NOW, &ts, BODY, without_prefix));
}

#[test]
fn invalid_hex_in_signature_fails() {
    let ts = NOW.to_string();
    assert!(!verify_request(
        SECRET,
        NOW,
        &ts,
        BODY,
        "v0=not_valid_hex!!"
    ));
}
