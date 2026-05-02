use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Returns `true` if the request timestamp is within 5 minutes of `now_unix` and the
/// HMAC-SHA256 signature over `v0:<timestamp>:<body>` matches `sig_header`.
pub fn verify_request(
    signing_secret: &str,
    now_unix: i64,
    timestamp: &str,
    body: &[u8],
    sig_header: &str,
) -> bool {
    timestamp_is_fresh(timestamp, now_unix)
        && verify_signature(signing_secret, timestamp, body, sig_header)
}

fn timestamp_is_fresh(timestamp: &str, now_unix: i64) -> bool {
    timestamp
        .parse::<i64>()
        .map(|ts| (now_unix - ts).abs() <= 300)
        .unwrap_or(false)
}

fn verify_signature(signing_secret: &str, timestamp: &str, body: &[u8], sig_header: &str) -> bool {
    let Some(expected_hex) = sig_header.strip_prefix("v0=") else {
        return false;
    };
    let Ok(expected_bytes) = hex::decode(expected_hex) else {
        return false;
    };
    let mut mac =
        HmacSha256::new_from_slice(signing_secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(b"v0:");
    mac.update(timestamp.as_bytes());
    mac.update(b":");
    mac.update(body);
    mac.verify_slice(&expected_bytes).is_ok()
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
