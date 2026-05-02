use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use fruit_dynamo_db::{
    community_repo::DynamoDbCommunityRepo, event_log_repo::DynamoDbEventLogRepo,
};
use lambda_http::{run, service_fn, Body, Error, Request, Response};

mod command;
mod error;
mod identity;
mod payload;
mod verify;

struct AppState {
    signing_secret: String,
    community_repo: DynamoDbCommunityRepo,
    event_log_repo: DynamoDbEventLogRepo,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let signing_secret =
        std::env::var("SLACK_SIGNING_SECRET").expect("SLACK_SIGNING_SECRET must be set");
    let table_name = std::env::var("TABLE_NAME").expect("TABLE_NAME must be set");

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    let state = Arc::new(AppState {
        signing_secret,
        community_repo: DynamoDbCommunityRepo::new(client.clone(), &table_name),
        event_log_repo: DynamoDbEventLogRepo::new(client, &table_name),
    });

    run(service_fn(move |event| handler(event, state.clone()))).await
}

async fn handler(event: Request, state: Arc<AppState>) -> Result<Response<Body>, Error> {
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

    if !verify::verify_request(
        &state.signing_secret,
        now_unix,
        timestamp,
        body_bytes,
        sig_header,
    ) {
        return Ok(Response::builder().status(401).body(Body::Empty)?);
    }

    let payload = match payload::SlashPayload::from_form(body_bytes) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("{{\"error\": \"{e}\"}}");
            return Ok(Response::builder()
                .status(400)
                .header("Content-Type", "application/json")
                .body(Body::Text(msg))?);
        }
    };

    let workspace_ns = identity::workspace_namespace(&payload.team_id);
    let community_id = identity::community_id_for(workspace_ns, &payload.channel_id);
    let member_id = identity::member_id_for(workspace_ns, &payload.user_id);

    let result = command::dispatch(
        &state.community_repo,
        &state.event_log_repo,
        community_id,
        member_id,
        &payload.user_name,
        workspace_ns,
        &payload.text,
    )
    .await;

    match result {
        Ok(json) => {
            let body = serde_json::to_string(&json).unwrap_or_default();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::Text(body))?)
        }
        Err(e) => {
            let msg = format!("{{\"error\": \"{e}\"}}");
            Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::Text(msg))?)
        }
    }
}
