/// Parsed `application/x-www-form-urlencoded` body sent by Slack for slash commands.
#[derive(Debug, serde::Deserialize)]
pub struct SlashPayload {
    /// The text typed after the command name (e.g. `"bag"` or `"gift <@U123> 🍎 hi"`).
    #[serde(default)]
    pub text: String,
    /// The Slack channel ID the command was invoked in.
    pub channel_id: String,
    /// The Slack user ID of the invoking user.
    pub user_id: String,
    /// The display name of the invoking user at invocation time.
    pub user_name: String,
    /// The Slack workspace (team) ID.
    pub team_id: String,
}

impl SlashPayload {
    /// Parses a Slack slash-command `application/x-www-form-urlencoded` POST body.
    pub fn from_form(body: &[u8]) -> Result<Self, serde_urlencoded::de::Error> {
        serde_urlencoded::from_bytes(body)
    }
}
