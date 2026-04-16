use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPC {
    #[serde(rename = "topic")]
    pub topic: String,

    #[serde(rename = "status")]
    pub status: Option<String>,

    #[serde(rename = "message")]
    pub message: Option<serde_json::Value>,

    #[serde(rename = "requestId", default)]
    pub request_id: String,

    #[serde(rename = "withMultipleParam")]
    pub with_multiple_param: Option<String>,
}

impl Default for IPC {
    fn default() -> Self {
        Self::new()
    }
}

impl IPC {
    pub fn new() -> Self {
        Self {
            topic: String::new(),
            status: None,
            message: None,
            request_id: String::new(),
            with_multiple_param: None,
        }
    }

    pub fn from_topic(topic: String) -> Self {
        Self {
            topic,
            message: None,
            request_id: Uuid::new_v4().to_string(),
            status: None,
            with_multiple_param: None,
        }
    }

    pub fn from_topic_message(topic: String, message: serde_json::Value) -> Self {
        Self {
            topic,
            message: Some(message),
            request_id: Uuid::new_v4().to_string(),
            status: None,
            with_multiple_param: None,
        }
    }

    pub fn from_topic_message_request_id(topic: String, message: Option<serde_json::Value>, request_id: String) -> Self {
        Self {
            topic,
            message,
            request_id,
            status: None,
            with_multiple_param: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_roundtrip() {
        let ipc = IPC::from_topic_message(
            "passbolt.test.topic".to_string(),
            serde_json::json!({"key": "value"}),
        );

        let json = serde_json::to_string(&ipc).unwrap();
        let deserialized: IPC = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.topic, "passbolt.test.topic");
        assert_eq!(deserialized.message, Some(serde_json::json!({"key": "value"})));
    }

    #[test]
    fn test_serialization_uses_camel_case() {
        let ipc = IPC::from_topic_message_request_id(
            "test".to_string(),
            None,
            "req-123".to_string(),
        );
        let json = serde_json::to_string(&ipc).unwrap();
        assert!(json.contains("requestId"));
        assert!(json.contains("withMultipleParam"));
        assert!(!json.contains("request_id"));
    }

    #[test]
    fn test_from_topic() {
        let ipc = IPC::from_topic("test.topic".to_string());
        assert_eq!(ipc.topic, "test.topic");
        assert!(ipc.message.is_none());
        assert!(!ipc.request_id.is_empty());
    }

    #[test]
    fn test_default() {
        let ipc = IPC::default();
        assert_eq!(ipc.topic, "");
        assert!(ipc.message.is_none());
        assert_eq!(ipc.request_id, "");
    }
}
