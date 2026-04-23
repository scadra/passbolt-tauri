//! Clipboard service — writes text to the OS clipboard via `arboard`.
//!
//! The secret never touches the Rendered WebView — it goes straight from
//! Background JS -> Rust -> OS clipboard.

/// Write text to the system clipboard.
///
/// Returns `Ok(())` on success, or an error message describing the failure.
pub fn write_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|error| format!("Failed to open clipboard: {}", error))?;

    clipboard.set_text(text)
        .map_err(|error| format!("Failed to write to clipboard: {}", error))
}
