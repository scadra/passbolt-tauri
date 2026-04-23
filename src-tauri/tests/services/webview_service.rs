use passbolt::services::webview_service::build_webview_url;
use passbolt::state::AppMode;

const TEST_UUID: &str = "abc12345-1234-5678-9abc-def012345678";

/// Rendered import URL uses UUID origin.
#[test]
fn rendered_import_url() {
    let url = build_webview_url("rendered-webview", TEST_UUID, &AppMode::Import);
    assert_eq!(url, format!("passbolt://{}.passbolt.local/apps/rendered-webview/index-import.html", TEST_UUID));
}

#[test]
fn rendered_auth_url() {
    let url = build_webview_url("rendered-webview", TEST_UUID, &AppMode::Auth);
    assert!(url.contains("index-auth.html"));
    assert!(url.contains(TEST_UUID));
}

#[test]
fn rendered_workspace_url() {
    let url = build_webview_url("rendered-webview", TEST_UUID, &AppMode::Workspace);
    assert!(url.contains("index-workspace.html"));
}

#[test]
fn background_import_url() {
    let url = build_webview_url("background-webview", TEST_UUID, &AppMode::Import);
    assert!(url.contains("background-webview"));
    assert!(url.contains("index-import.html"));
}

#[test]
fn background_auth_url() {
    let url = build_webview_url("background-webview", TEST_UUID, &AppMode::Auth);
    assert!(url.contains("background-webview"));
    assert!(url.contains("index-auth.html"));
}

#[test]
fn background_workspace_url() {
    let url = build_webview_url("background-webview", TEST_UUID, &AppMode::Workspace);
    assert!(url.contains("index-workspace.html"));
}

/// All URLs use the `passbolt://` scheme.
#[test]
fn all_urls_use_passbolt_scheme() {
    for mode in [AppMode::Import, AppMode::Auth, AppMode::Workspace] {
        for target in ["rendered-webview", "background-webview"] {
            let url = build_webview_url(target, TEST_UUID, &mode);
            assert!(url.starts_with("passbolt://"));
        }
    }
}

/// All URLs contain the UUID and domain suffix.
#[test]
fn all_urls_contain_uuid_and_domain() {
    for mode in [AppMode::Import, AppMode::Auth, AppMode::Workspace] {
        let url = build_webview_url("rendered-webview", TEST_UUID, &mode);
        assert!(url.contains(&format!("{}.passbolt.local", TEST_UUID)));
    }
}
