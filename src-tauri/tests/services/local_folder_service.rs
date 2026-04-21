use passbolt_app::services::local_folder_service::{self, IndexContext, WebviewTarget};
use passbolt_app::state::AppMode;

const TEST_UUID: &str = "abc12345-1234-5678-9abc-def012345678";
const TEST_DOMAIN: &str = "https://passbolt.example.com";
const TEST_ISOLATION: &str = "isolation-870dae28-821d-4ad5-8c80-2dd6846f1fbe";

fn ctx<'a>(
    target: &'a WebviewTarget,
    mode: &'a AppMode,
    trusted_domain: Option<&'a str>,
    isolation_schema: Option<&'a str>,
) -> IndexContext<'a> {
    IndexContext { target, mode, uuid: TEST_UUID, trusted_domain, isolation_schema }
}

// ── HTML generation ─────────────────────────────────────────────

#[test]
fn rendered_import_html_contains_import_bundle() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(html.contains("rendered-import.js"));
}

#[test]
fn rendered_auth_html_contains_auth_bundle() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Rendered, &AppMode::Auth, Some(TEST_DOMAIN), None));
    assert!(html.contains("rendered-auth.js"));
}

#[test]
fn rendered_workspace_html_contains_workspace_bundle() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Rendered, &AppMode::Workspace, Some(TEST_DOMAIN), None));
    assert!(html.contains("rendered-workspace.js"));
}

#[test]
fn background_import_html_contains_import_bundle() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Background, &AppMode::Import, None, None));
    assert!(html.contains("background-import.js"));
}

#[test]
fn background_auth_html_contains_auth_bundle() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Background, &AppMode::Auth, Some(TEST_DOMAIN), None));
    assert!(html.contains("background-auth.js"));
}

#[test]
fn background_workspace_html_contains_workspace_bundle() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Background, &AppMode::Workspace, Some(TEST_DOMAIN), None));
    assert!(html.contains("background-workspace.js"));
}

#[test]
fn generated_html_has_doctype() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(html.starts_with("<!DOCTYPE html>"));
}

#[test]
fn generated_html_has_utf8_charset() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Background, &AppMode::Auth, Some(TEST_DOMAIN), None));
    assert!(html.contains(r#"charset="UTF-8""#));
}

/// The HTML must include the instance UUID as a meta tag.
#[test]
fn generated_html_has_instance_uuid() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(html.contains(TEST_UUID));
}

/// Bundle URLs must include the UUID-based origin.
#[test]
fn bundle_src_uses_uuid_origin() {
    let html = local_folder_service::generate_index_html(&ctx(&WebviewTarget::Rendered, &AppMode::Auth, Some(TEST_DOMAIN), None));
    assert!(html.contains(&format!("passbolt://{}.passbolt.local/", TEST_UUID)));
}

// ── Isolation frame-src ─────────────────────────────────────────

/// With an isolation schema, the CSP must contain `frame-src {schema}://localhost`.
#[test]
fn csp_has_isolation_frame_src_when_provided() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, Some(TEST_ISOLATION)));
    assert!(csp.contains(&format!("frame-src {}://localhost", TEST_ISOLATION)));
}

/// Without an isolation schema, no `frame-src` directive.
#[test]
fn csp_has_no_frame_src_without_isolation() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(!csp.contains("frame-src"));
}

// ── IPC connect-src ─────────────────────────────────────────────

/// All CSPs must contain IPC paths for Tauri event system.
#[test]
fn all_csp_have_ipc_connect_src() {
    for target in [WebviewTarget::Rendered, WebviewTarget::Background] {
        for mode in [AppMode::Import, AppMode::Auth, AppMode::Workspace] {
            let csp = local_folder_service::generate_csp(&ctx(&target, &mode, Some(TEST_DOMAIN), None));
            assert!(
                csp.contains("ipc://localhost/plugin:event|listen"),
                "{:?}/{:?} CSP missing ipc listen: {}", target, mode, csp
            );
            assert!(
                csp.contains("ipc://localhost/plugin:event|emit"),
                "{:?}/{:?} CSP missing ipc emit: {}", target, mode, csp
            );
        }
    }
}

// ── Rendered CSP ────────────────────────────────────────────────

#[test]
fn rendered_import_csp_has_no_trusted_domain_in_connect_src() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(!csp.contains(TEST_DOMAIN));
}

#[test]
fn rendered_auth_csp_has_locales_in_connect_src() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Auth, Some(TEST_DOMAIN), None));
    assert!(csp.contains("locales/"));
}

#[test]
fn rendered_csp_never_includes_api_domain_in_connect_src() {
    for mode in [AppMode::Import, AppMode::Auth, AppMode::Workspace] {
        let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &mode, Some(TEST_DOMAIN), None));
        if let Some(connect_part) = csp.split("; ").find(|p| p.starts_with("connect-src")) {
            assert!(
                !connect_part.contains(TEST_DOMAIN),
                "Rendered {:?} connect-src must not contain API domain: {}", mode, connect_part
            );
        }
    }
}

#[test]
fn rendered_auth_csp_has_trusted_domain_in_img_src() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Auth, Some(TEST_DOMAIN), None));
    let img_part = csp.split("; ").find(|p| p.starts_with("img-src")).unwrap();
    assert!(img_part.contains(TEST_DOMAIN));
}

#[test]
fn rendered_workspace_csp_has_data_and_blob_in_img_src() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Workspace, Some(TEST_DOMAIN), None));
    let img_part = csp.split("; ").find(|p| p.starts_with("img-src")).unwrap();
    assert!(img_part.contains("data:"));
    assert!(img_part.contains("blob:"));
}

#[test]
fn rendered_csp_has_default_src_none() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Auth, Some(TEST_DOMAIN), None));
    assert!(csp.contains("default-src 'none'"));
}

#[test]
fn rendered_csp_has_form_action_none() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Auth, Some(TEST_DOMAIN), None));
    assert!(csp.contains("form-action 'none'"));
}

#[test]
fn rendered_csp_has_script_src_with_uuid() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(csp.contains(&format!("script-src passbolt://{}.passbolt.local/apps/rendered-webview/", TEST_UUID)));
}

#[test]
fn rendered_csp_has_style_src_with_themes() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(csp.contains("themes/"));
}

#[test]
fn rendered_csp_has_font_src() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Rendered, &AppMode::Import, None, None));
    assert!(csp.contains("font-src"));
    assert!(csp.contains("fonts/"));
}

// ── Background CSP ──────────────────────────────────────────────

#[test]
fn background_import_csp_has_no_trusted_domain() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Background, &AppMode::Import, None, None));
    assert!(!csp.contains(TEST_DOMAIN));
}

#[test]
fn background_auth_csp_has_no_trusted_domain_in_connect_src() {
    // All HTTP goes through IPC proxy — trusted domain not needed in CSP
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Background, &AppMode::Auth, Some(TEST_DOMAIN), None));
    let connect_part = csp.split("; ").find(|p| p.starts_with("connect-src")).unwrap();
    assert!(!connect_part.contains(TEST_DOMAIN));
}

#[test]
fn background_workspace_csp_has_no_pwnedpasswords() {
    // HIBP requests go through IPC proxy — not needed in CSP
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Background, &AppMode::Workspace, Some(TEST_DOMAIN), None));
    assert!(!csp.contains("pwnedpasswords"));
}

#[test]
fn background_csp_has_default_src_none() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Background, &AppMode::Import, None, None));
    assert!(csp.contains("default-src 'none'"));
}

#[test]
fn background_csp_has_script_src_with_uuid() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Background, &AppMode::Import, None, None));
    assert!(csp.contains(&format!("script-src passbolt://{}.passbolt.local/apps/background-webview/", TEST_UUID)));
}

#[test]
fn background_csp_has_locales_in_connect_src() {
    let csp = local_folder_service::generate_csp(&ctx(&WebviewTarget::Background, &AppMode::Import, None, None));
    let connect_part = csp.split("; ").find(|p| p.starts_with("connect-src")).unwrap();
    assert!(connect_part.contains("locales/"));
}
