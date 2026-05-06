use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anomalies::category::{Busy, Conflict, Forbidden, Interrupted, NotFound, Unavailable};
use exn::Exn;
use fruit_domain::{community_store::CommunityStore, event_log_store::EventLogStore};
use fruit_dynamo_db::{
    community_repo::DynamoDbCommunityRepo, event_log_repo::DynamoDbEventLogRepo,
};
use lambda_runtime::{run, service_fn, LambdaEvent};
use notify::HttpSlackNotifier;
use serde_json::{json, Value};
use tracing::debug;

mod command;
mod error;
mod grant;
mod identity;
mod notify;
mod payload;
mod verify;

struct AppState<N: notify::Notifier> {
    signing_secret: String,
    community_repo: DynamoDbCommunityRepo,
    event_log_repo: DynamoDbEventLogRepo,
    notifier: N,
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let signing_secret =
        std::env::var("SLACK_SIGNING_SECRET").expect("SLACK_SIGNING_SECRET must be set");
    let bot_token = std::env::var("SLACK_BOT_TOKEN").expect("SLACK_BOT_TOKEN must be set");
    let table_name = std::env::var("TABLE_NAME").expect("TABLE_NAME must be set");

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_dynamodb::Client::new(&config);

    let state = Arc::new(AppState {
        signing_secret,
        community_repo: DynamoDbCommunityRepo::new(client.clone(), &table_name),
        event_log_repo: DynamoDbEventLogRepo::new(client, &table_name),
        notifier: HttpSlackNotifier::new(bot_token),
    });

    run(service_fn(move |event: LambdaEvent<Value>| {
        let state = state.clone();
        async move { handler(event.payload, state).await }
    }))
    .await
}

async fn handler<N: notify::Notifier>(
    event: Value,
    state: Arc<AppState<N>>,
) -> Result<Value, lambda_runtime::Error> {
    if is_event_bridge(&event) {
        handle_event_bridge(event, &state).await
    } else {
        handle_http(event, &state).await
    }
}

/// Returns `true` when the Lambda payload looks like an EventBridge event.
fn is_event_bridge(event: &Value) -> bool {
    event.get("detail-type").is_some()
}

// ── EventBridge path ──────────────────────────────────────────────────────────

async fn handle_event_bridge<N: notify::Notifier>(
    event: Value,
    state: &AppState<N>,
) -> Result<Value, lambda_runtime::Error> {
    let detail: grant::GrantDetail = serde_json::from_value(event["detail"].clone())?;

    let community_store = CommunityStore::new(&state.community_repo, &state.event_log_repo);

    let result = grant::handle_grant(
        &community_store,
        &state.community_repo,
        &state.event_log_repo,
        &state.notifier,
        &detail,
    )
    .await;

    match result {
        Ok(()) => Ok(json!({})),
        Err(e) => {
            debug!("{:?}", e);
            Err(format!("{e:?}").into())
        }
    }
}

// ── HTTP (API Gateway v2) path ────────────────────────────────────────────────

async fn handle_http<N: notify::Notifier>(
    event: Value,
    state: &AppState<N>,
) -> Result<Value, lambda_runtime::Error> {
    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);

    // API Gateway v2 lowercases all header names.
    let timestamp = event["headers"]["x-slack-request-timestamp"]
        .as_str()
        .unwrap_or("");
    let sig_header = event["headers"]["x-slack-signature"].as_str().unwrap_or("");
    let body_str = event["body"].as_str().unwrap_or("");
    let body_bytes = body_str.as_bytes();

    if !verify::verify_request(
        &state.signing_secret,
        now_unix,
        timestamp,
        body_bytes,
        sig_header,
    ) {
        return Ok(json!({"statusCode": 401, "body": ""}));
    }

    let payload = match payload::SlashPayload::from_form(body_bytes) {
        Ok(p) => p,
        Err(e) => {
            let body = format!("{{\"error\": \"{e}\"}}");
            return Ok(json!({
                "statusCode": 400,
                "headers": {"content-type": "application/json"},
                "body": body,
            }));
        }
    };

    let workspace_ns = identity::workspace_namespace(&payload.team_id);
    let community_id = identity::community_id_for(workspace_ns, &payload.channel_id);
    let member_id = identity::member_id_for(workspace_ns, &payload.user_id);

    let community_store = CommunityStore::new(&state.community_repo, &state.event_log_repo);
    let event_log_store = EventLogStore::new(&state.event_log_repo);

    let result = command::dispatch(
        &community_store,
        &event_log_store,
        community_id,
        member_id,
        &payload.user_name,
        workspace_ns,
        &payload.text,
    )
    .await;

    match result {
        Ok(json_val) => {
            let body = serde_json::to_string(&json_val).unwrap_or_default();
            Ok(json!({
                "statusCode": 200,
                "headers": {"content-type": "application/json"},
                "body": body,
            }))
        }
        Err(e) => {
            debug!("{:?}", e);
            let status = command_error_status(&e);
            let body = format!("{{\"error\": \"{e}\"}}");
            Ok(json!({
                "statusCode": status,
                "headers": {"content-type": "application/json"},
                "body": body,
            }))
        }
    }
}

/// Maps a command error's anomaly category to an HTTP status code.
#[allow(non_upper_case_globals)]
fn command_error_status(e: &Exn<crate::error::Error>) -> u16 {
    use anomalies::anomaly::HasCategory;
    match e.category() {
        Unavailable | Busy | Interrupted => 503,
        Forbidden => 403,
        NotFound => 404,
        Conflict => 409,
        _ => 500,
    }
}
