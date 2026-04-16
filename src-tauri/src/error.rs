use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Unauthorized topic from {origin}: {topic}")]
    UnauthorizedTopic { origin: String, topic: String },

    #[error(transparent)]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Asset not found: {path}")]
    AssetNotFound { path: String },

    #[error("Webview error: {0}")]
    Webview(String),

    #[error("IPC routing error: {0}")]
    IpcRouting(String),

    #[error("Credential storage error: {0}")]
    Credential(String),

    #[error(transparent)]
    UrlParse(#[from] url::ParseError),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}
