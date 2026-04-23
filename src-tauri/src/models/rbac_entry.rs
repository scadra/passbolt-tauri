use serde::Serialize;

/// A single RBAC permission entry, matching the shape returned by the
/// `passbolt.rbacs.find-me` API endpoint.
///
/// Used by `rbac_service` to inject desktop-specific Deny rules.
#[derive(Debug, Serialize)]
pub struct RbacEntry {
    pub id: String,
    pub foreign_id: String,
    pub role_id: String,
    pub control_function: String,
    pub foreign_model: String,
    pub action: RbacAction,
    pub ui_action: RbacAction,
}

/// Nested action/ui_action object within an RBAC entry.
#[derive(Debug, Serialize)]
pub struct RbacAction {
    pub id: String,
    pub name: String,
}

impl RbacEntry {
    /// Create a new RBAC entry with generated UUIDs for a given ui_action name
    /// and control function (e.g. "Deny" or "Allow").
    pub fn new(ui_action_name: &str, control_function: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            foreign_id: uuid::Uuid::new_v4().to_string(),
            role_id: uuid::Uuid::new_v4().to_string(),
            control_function: control_function.to_string(),
            foreign_model: "UiAction".to_string(),
            action: RbacAction {
                id: uuid::Uuid::new_v4().to_string(),
                name: ui_action_name.to_string(),
            },
            ui_action: RbacAction {
                id: uuid::Uuid::new_v4().to_string(),
                name: ui_action_name.to_string(),
            },
        }
    }
}
