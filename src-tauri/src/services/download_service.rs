//! File download service — decodes a Data URL and saves to ~/Downloads.
//!
//! Mirrors the Windows UWP `DownloadService` behavior:
//! - No "Save As" dialog — writes directly to the Downloads folder.
//! - `GenerateUniqueName` collision handling: `file (1).csv`, `file (2).csv`, etc.

use base64::Engine;
use log::{info, warn};
use std::path::{Path, PathBuf};

/// Decode a Data URL and save the resulting bytes to ~/Downloads.
///
/// `content` is a Data URL: `data:mime;base64,XXXX`.
/// `filename` is the suggested file name (e.g. `passbolt-export-2026-04-22.csv`).
pub fn download(content: &str, filename: &str) {
    // Parse data URL: split on first comma to separate header from base64 payload
    let base64_data = match content.split_once(',') {
        Some((_header, data)) => data,
        None => content,
    };

    let bytes = match base64::engine::general_purpose::STANDARD.decode(base64_data) {
        Ok(bytes) => bytes,
        Err(error) => {
            warn!("[DownloadService] Base64 decode failed: {}", error);
            return;
        }
    };

    let downloads_dir = match dirs::download_dir() {
        Some(dir) => dir,
        None => {
            warn!("[DownloadService] Cannot find Downloads folder");
            return;
        }
    };

    let safe_filename = sanitize_filename(filename);
    let target_path = unique_path(&downloads_dir, &safe_filename);

    match std::fs::write(&target_path, &bytes) {
        Ok(_) => info!("[DownloadService] Saved: {} ({} bytes)", target_path.display(), bytes.len()),
        Err(error) => warn!("[DownloadService] Write failed: {}", error),
    }
}

/// Sanitize a filename by extracting only the final component, stripping
/// directory separators and parent-directory references.
///
/// Security: prevents path traversal attacks where a malicious filename
/// like `../../../etc/something` could escape the Downloads directory.
fn sanitize_filename(raw_filename: &str) -> String {
    // Use Path to extract only the final component (handles both / and \ separators)
    let file_component = std::path::Path::new(raw_filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");

    // Belt-and-suspenders: reject if it still contains path traversal sequences
    let sanitized = file_component.replace("..", "");

    if sanitized.is_empty() || sanitized == "." {
        return "download".to_string();
    }

    sanitized
}

/// Generate a unique file path, appending (1), (2), etc. on collision.
/// Same as Windows `CreationCollisionOption.GenerateUniqueName`.
fn unique_path(dir: &Path, filename: &str) -> PathBuf {
    let path = dir.join(filename);
    if !path.exists() {
        return path;
    }

    let stem = Path::new(filename)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(filename);
    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str());

    for counter in 1..100 {
        let new_name = match extension {
            Some(ext) => format!("{} ({}).{}", stem, counter, ext),
            None => format!("{} ({})", stem, counter),
        };
        let new_path = dir.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_normal_filename() {
        assert_eq!(sanitize_filename("passbolt-export.csv"), "passbolt-export.csv");
    }

    #[test]
    fn test_sanitize_path_traversal_unix() {
        assert_eq!(sanitize_filename("../../../etc/passwd"), "passwd");
    }

    #[test]
    fn test_sanitize_path_traversal_relative() {
        assert_eq!(sanitize_filename("../../secret.txt"), "secret.txt");
    }

    #[test]
    fn test_sanitize_dotdot_only() {
        assert_eq!(sanitize_filename(".."), "download");
    }

    #[test]
    fn test_sanitize_empty_string() {
        assert_eq!(sanitize_filename(""), "download");
    }

    #[test]
    fn test_sanitize_absolute_path() {
        assert_eq!(sanitize_filename("/etc/passwd"), "passwd");
    }

    #[test]
    fn test_sanitize_preserves_extension() {
        assert_eq!(sanitize_filename("../report.pdf"), "report.pdf");
    }
}
