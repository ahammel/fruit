/// Service-level error for the slash-command handler.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A storage operation failed.
    #[error("storage error: {0}")]
    Storage(String),
}
