/// Application modes — determines which JS bundle each webview loads.
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Import,
    Auth,
    Workspace,
}
