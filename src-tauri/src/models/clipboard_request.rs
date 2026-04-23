use serde::Deserialize;

/// Payload for clipboard write requests.
///
/// JS may send a plain string or an object with a `text` field.
/// Array-unwrapping is handled by `unwrap_ipc_array` before deserialization.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ClipboardRequest {
    Plain(String),
    Structured { text: String },
}

impl ClipboardRequest {
    /// Extract the text to copy to the clipboard.
    pub fn text(&self) -> &str {
        match self {
            ClipboardRequest::Plain(value) => value,
            ClipboardRequest::Structured { text } => text,
        }
    }
}
