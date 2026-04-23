use serde::Deserialize;

/// Payload for file download requests.
///
/// `content` is a Data URL (`data:mime;base64,XXXX`).
/// `filename` is optional — defaults to "download" if absent.
#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    #[serde(default = "default_filename")]
    pub filename: String,
    pub content: String,
}

fn default_filename() -> String {
    "download".to_string()
}
