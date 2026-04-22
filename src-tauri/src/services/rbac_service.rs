//! RBAC service — injects desktop-specific Deny rules into RBAC responses.
//!
//! The desktop app does not support all features available in the web app.
//! When the Rendered webview requests `passbolt.rbacs.find-me`, the API
//! returns the user's RBAC configuration. This service intercepts the
//! response and force-denies features unsupported on desktop.
//!
//! Same list as WinUI 3's `RbacService.AddDesktopRbac`.

use serde_json::Value;

/// Features forced to Deny on the desktop app.
const DESKTOP_DENY_ACTIONS: &[&str] = &[
    "Administration.viewWorkspace",
    "Duo.configuration",
    "Avatar.upload",
    "Mobile.transfer",
    "Desktop.transfer",
    "Profil.accountRecovery",
];

/// Inject desktop Deny rules into an RBAC response message.
pub fn inject_desktop_denies(message: &mut Option<Value>) {
    if let Some(ref mut value) = message {
        if let Some(arr) = value.as_array_mut() {
            for action_name in DESKTOP_DENY_ACTIONS {
                add_or_update(arr, action_name, "Deny");
            }
        }
    }
}

/// Add or update an RBAC entry. If the ui_action already exists, overwrite
/// its control_function. Otherwise create a new entry.
fn add_or_update(rbacs: &mut Vec<Value>, ui_action_name: &str, control: &str) {
    let existing = rbacs.iter_mut().find(|r| {
        r.get("ui_action")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            == Some(ui_action_name)
    });

    if let Some(entry) = existing {
        entry["control_function"] = Value::String(control.to_string());
    } else {
        rbacs.push(serde_json::json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "foreign_id": uuid::Uuid::new_v4().to_string(),
            "role_id": uuid::Uuid::new_v4().to_string(),
            "control_function": control,
            "foreign_model": "UiAction",
            "action": {
                "id": uuid::Uuid::new_v4().to_string(),
                "name": ui_action_name
            },
            "ui_action": {
                "id": uuid::Uuid::new_v4().to_string(),
                "name": ui_action_name
            }
        }));
    }
}
