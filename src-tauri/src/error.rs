use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Unauthorized topic: {0}")]
    UnauthorizedTopic(String),

    #[error("Invalid request ID: {0}")]
    InvalidRequestId(String),

    #[error("Credential error: {0}")]
    CredentialError(String),

    #[error("Webview not found: {0}")]
    WebviewNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Tauri error: {0}")]
    TauriError(#[from] tauri::Error),
}
