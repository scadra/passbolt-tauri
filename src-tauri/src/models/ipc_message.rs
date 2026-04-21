use serde::{Deserialize, Serialize};

/// IPC wire envelope — the single message shape exchanged between JS and Rust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub topic: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,

    #[serde(default, alias = "requestId", rename = "requestId")]
    pub request_id: Option<String>,

    #[serde(default, rename = "withMultipleParam")]
    pub with_multiple_param: bool,
}

impl IpcMessage {
    /// Create a simple message with a topic and optional payload.
    pub fn new(topic: &str, message: Option<serde_json::Value>) -> Self {
        Self {
            topic: topic.to_string(),
            status: None,
            message,
            request_id: None,
            with_multiple_param: false,
        }
    }

    /// Create a success response for a given request ID.
    pub fn success(request_id: &str, message: Option<serde_json::Value>) -> Self {
        Self {
            topic: request_id.to_string(),
            status: Some("SUCCESS".to_string()),
            message,
            request_id: Some(request_id.to_string()),
            with_multiple_param: false,
        }
    }

    /// Create an error response for a given request ID.
    pub fn error(request_id: &str, message: Option<serde_json::Value>) -> Self {
        Self {
            topic: request_id.to_string(),
            status: Some("ERROR".to_string()),
            message,
            request_id: Some(request_id.to_string()),
            with_multiple_param: false,
        }
    }
}
