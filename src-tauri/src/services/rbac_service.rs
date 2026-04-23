//! RBAC service — injects desktop-specific Deny rules into RBAC responses.
//!
//! The desktop app does not support all features available in the web app.
//! When the Rendered webview requests `passbolt.rbacs.find-me`, the API
//! returns the user's RBAC configuration. This service intercepts the
//! response and force-denies features unsupported on desktop.
//!
//! Same list as WinUI 3's `RbacService.AddDesktopRbac`.

use serde_json::Value;

use crate::models::rbac_entry::RbacEntry;

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
        if let Some(rbacs) = value.as_array_mut() {
            for action_name in DESKTOP_DENY_ACTIONS {
                add_or_update(rbacs, action_name, "Deny");
            }
        }
    }
}

/// Add or update an RBAC entry. If the ui_action already exists, overwrite
/// its control_function. Otherwise create a new entry using `RbacEntry`.
fn add_or_update(rbacs: &mut Vec<Value>, ui_action_name: &str, control: &str) {
    let existing = rbacs.iter_mut().find(|entry| {
        entry.get("ui_action")
            .and_then(|action| action.get("name"))
            .and_then(|name| name.as_str())
            == Some(ui_action_name)
    });

    if let Some(entry) = existing {
        entry["control_function"] = Value::String(control.to_string());
    } else {
        let new_entry = RbacEntry::new(ui_action_name, control);
        if let Ok(serialized) = serde_json::to_value(new_entry) {
            rbacs.push(serialized);
        }
    }
}
