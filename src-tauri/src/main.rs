use log::{info, error, debug};
use tauri::Manager;

use passbolt_app::config;
use passbolt_app::services::{ipc_listener_service, webview_service};
use passbolt_app::services::protocol_service::ProtocolService;
use passbolt_app::state::{AppMode, AppState};

fn main() {
    env_logger::init();

    let context = tauri::generate_context!();

    // Capture the isolation schema before context is consumed by .run()
    let isolation_schema = if let tauri::Pattern::Isolation { schema, .. } = context.pattern() {
        Some(schema.to_string())
    } else {
        None
    };

    let protocol_service = ProtocolService::new(isolation_schema.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![debug_ping])
        // passbolt:// — serves embedded webview assets (HTML, JS, CSS, fonts)
        .register_asynchronous_uri_scheme_protocol(config::PROTOCOL_SCHEME, move |ctx, request, responder| {
            let url = request.uri().to_string();
            let state = ctx.app_handle().state::<AppState>();
            let response = protocol_service.handle_request(&url, &state);
            responder.respond(response);
        })
        .setup(move |app| {
            info!("Passbolt Desktop starting...");

            // Store isolation schema in state
            if let Some(ref schema) = isolation_schema {
                let state = app.state::<AppState>();
                state.navigation.set_isolation_schema(schema.clone());
            }

            debug!("[Setup] Determining initial mode...");
            let mode = determine_initial_mode(app);

            debug!("[Setup] Setting up IPC listeners...");
            ipc_listener_service::setup(app);

            debug!("[Setup] Creating webviews...");
            let state = app.state::<AppState>();
            let rv_uuid = state.navigation.get_rendered_uuid();
            let bg_uuid = state.navigation.get_background_uuid();

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                debug!("[Setup] Async: creating webviews...");
                if let Err(e) = webview_service::create_webviews(&app_handle, &mode, &rv_uuid, &bg_uuid) {
                    error!("[Setup] Failed to create webviews: {}", e);
                }
                debug!("[Setup] Async: webviews created");
            });

            debug!("[Setup] Setup complete, returning to event loop");

            Ok(())
        })
        .run(context)
        .expect("error while running tauri application");
}

#[tauri::command]
fn debug_ping() -> String {
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG_PING] pong");
    "pong".into()
}

/// Decide which mode to boot into based on whether an account exists in the keyring.
fn determine_initial_mode(app: &tauri::App) -> AppMode {
    use passbolt_app::services::keyring_service::KeyringService;

    let keyring = KeyringService::default();
    let state = app.state::<AppState>();
    let mode = if let Some(metadata) = keyring.get_account_metadata() {
        state.credentials.set_metadata(metadata.clone());
        state.navigation.set_trusted_domain(metadata.domain.clone());
        // Also cache secret to avoid a second keyring prompt
        if let Some(secret) = keyring.get_account_secret() {
            state.credentials.set_secret(secret);
        }
        AppMode::Auth
    } else {
        AppMode::Import
    };

    let state = app.state::<AppState>();
    state.navigation.set_mode(mode.clone());

    info!("Initial mode: {:?}", mode);
    mode
}
