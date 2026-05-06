use exn::Exn;
use reqwest::Client;
use serde_json::json;

use crate::error::{Error, NotificationError};

const CHAT_POST_MESSAGE_URL: &str = "https://slack.com/api/chat.postMessage";

/// Posts messages to Slack channels.
pub trait Notifier: Send + Sync {
    /// Posts `text` to the given Slack `channel_id`.
    async fn post_message(&self, channel_id: &str, text: &str) -> Result<(), Exn<Error>>;
}

/// [`Notifier`] that calls the live Slack `chat.postMessage` API.
pub struct HttpSlackNotifier {
    client: Client,
    bot_token: String,
}

impl HttpSlackNotifier {
    /// Creates a new `HttpSlackNotifier` using the given bot token.
    pub fn new(bot_token: String) -> Self {
        Self {
            client: Client::new(),
            bot_token,
        }
    }
}

impl Notifier for HttpSlackNotifier {
    async fn post_message(&self, channel_id: &str, text: &str) -> Result<(), Exn<Error>> {
        let resp = self
            .client
            .post(CHAT_POST_MESSAGE_URL)
            .bearer_auth(&self.bot_token)
            .json(&json!({"channel": channel_id, "text": text}))
            .send()
            .await
            .map_err(|e| NotificationError::network("chat.postMessage request failed", e))?;

        let body: serde_json::Value = resp.json().await.map_err(|e| {
            NotificationError::network("failed to read chat.postMessage response", e)
        })?;

        if body["ok"].as_bool().unwrap_or(false) {
            Ok(())
        } else {
            let api_err = body["error"].as_str().unwrap_or("unknown").to_string();
            Err(NotificationError::slack_api(api_err))
        }
    }
}
