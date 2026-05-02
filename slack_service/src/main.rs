use std::time::{SystemTime, UNIX_EPOCH};

use lambda_http::{run, service_fn, Body, Error, Request, Response};

mod verify;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let signing_secret =
        std::env::var("SLACK_SIGNING_SECRET").expect("SLACK_SIGNING_SECRET must be set");
    run(service_fn(move |event| {
        handler(event, signing_secret.clone())
    }))
    .await
}

async fn handler(event: Request, signing_secret: String) -> Result<Response<Body>, Error> {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);

    let headers = event.headers();
    let timestamp = headers
        .get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let sig_header = headers
        .get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let body_bytes: &[u8] = match event.body() {
        Body::Empty => &[],
        Body::Text(s) => s.as_bytes(),
        Body::Binary(b) => b.as_slice(),
    };

    if !verify::verify_request(&signing_secret, now_unix, timestamp, body_bytes, sig_header) {
        return Ok(Response::builder().status(401).body(Body::Empty)?);
    }

    todo!()
}
