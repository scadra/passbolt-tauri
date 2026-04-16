use std::sync::{Arc, Mutex};
use tauri::{App, WebviewUrl, WebviewWindowBuilder, Listener};
use uuid::Uuid;
use tauri::Manager;
use log::{info, error, debug};

mod plugins;

use passbolt_app::AppResult;
use passbolt_app::config;
use passbolt_app::state::AppState;
use passbolt_app::services::local_folder_service::LocalFolderService;
use passbolt_app::services::protocol_service::ProtocolService;
use passbolt_app::services::rendered_topic_service::RenderedTopicService;
use passbolt_app::services::background_topic_service::BackgroundTopic;
use passbolt_app::models::ipc_model::IPC;
use passbolt_app::models::messaging::AllowedTopics;

async fn initialize_app(app: tauri::AppHandle) -> AppResult<()> {
    info!("Application launched");

    let background_uuid = Uuid::new_v4().to_string();
    let rendered_uuid = Uuid::new_v4().to_string();

    let app_state = app.state::<Arc<AppState>>();
    app_state.set_background_uuid(&background_uuid);
    app_state.set_rendered_uuid(&rendered_uuid);

    debug!("Background domain: {}://{}{}", config::PROTOCOL_SCHEME, background_uuid, config::DOMAIN_SUFFIX);
    debug!("Rendered domain: {}://{}{}", config::PROTOCOL_SCHEME, rendered_uuid, config::DOMAIN_SUFFIX);

    let _background_webview = WebviewWindowBuilder::new(
        &app,
        config::BACKGROUND_WEBVIEW_LABEL,
        WebviewUrl::CustomProtocol(
            format!("{}://{}{}/background", config::PROTOCOL_SCHEME, background_uuid, config::DOMAIN_SUFFIX)
                .parse()?
        )
    )
    .title("Passbolt Background")
    .visible(false)
    .resizable(false)
    .build()?;

    let _rendered_webview = WebviewWindowBuilder::new(
        &app,
        config::RENDERED_WEBVIEW_LABEL,
        WebviewUrl::CustomProtocol(
            format!("{}://{}{}/app", config::PROTOCOL_SCHEME, rendered_uuid, config::DOMAIN_SUFFIX)
                .parse()?
        )
    )
    .title("Passbolt Desktop")
    .visible(true)
    .resizable(true)
    .inner_size(config::WINDOW_DEFAULT_WIDTH, config::WINDOW_DEFAULT_HEIGHT)
    .min_inner_size(config::WINDOW_MIN_WIDTH, config::WINDOW_MIN_HEIGHT)
    .build()?;

    #[cfg(debug_assertions)]
    {
        if let Some(window) = app.get_webview_window(config::BACKGROUND_WEBVIEW_LABEL) {
            window.open_devtools();
        }
        if let Some(window) = app.get_webview_window(config::RENDERED_WEBVIEW_LABEL) {
            window.open_devtools();
        }
    }

    info!("Application initialized successfully");
    Ok(())
}

fn setup_ipc_listeners(
    app: &App,
    rendered_service: Arc<Mutex<RenderedTopicService>>,
    background_service: Arc<Mutex<BackgroundTopic>>,
    app_state: Arc<AppState>,
) -> AppResult<()> {
    // Rendered → Background: messages from the rendered webview
    {
        let service = rendered_service.clone();
        let bg_service = background_service.clone();
        let state = app_state.clone();
        app.listen(config::RENDERED_IPC_EVENT, move |event| {
            let payload = event.payload();
            match serde_json::from_str::<IPC>(payload) {
                Ok(ipc) => {
                    // When rendered webview signals it's ready, update background service state
                    if ipc.topic == AllowedTopics::RENDERED_READY {
                        let mut bg_guard = bg_service.lock().unwrap_or_else(|e| e.into_inner());
                        bg_guard.set_rendered_ready(true);
                        bg_guard.process_pending_messages(&state);
                    }

                    let mut guard = service.lock().unwrap_or_else(|e| e.into_inner());
                    if let Err(e) = guard.proceed_message(ipc, &state) {
                        error!("[Rendered → Background] Error: {}", e);
                    }
                }
                Err(e) => error!("[Rendered] Failed to parse IPC: {}", e),
            }
        });
    }

    // Background → Rendered: messages from the background webview
    {
        let service = background_service.clone();
        let state = app_state.clone();
        app.listen(config::BACKGROUND_IPC_EVENT, move |event| {
            let payload = event.payload();
            match serde_json::from_str::<IPC>(payload) {
                Ok(ipc) => {
                    let mut guard = service.lock().unwrap_or_else(|e| e.into_inner());
                    if let Err(e) = guard.proceed_message(ipc, &state) {
                        error!("[Background → Rendered] Error: {}", e);
                    }
                }
                Err(e) => error!("[Background] Failed to parse IPC: {}", e),
            }
        });
    }

    Ok(())
}

pub fn run() {
    env_logger::init();

    let service = LocalFolderService::new();
    if let Err(e) = service.generate_isolation_html() {
        error!("Failed to generate isolation: {}", e);
        std::process::exit(1);
    }

    let context = tauri::generate_context!();
    let isolation_schema = if let tauri::Pattern::Isolation { schema, .. } = context.pattern() {
        Some(schema.to_string())
    } else {
        None
    };

    let protocol_service = ProtocolService::new(service, isolation_schema);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(plugins::navigation::init())
        .register_uri_scheme_protocol("passbolt", move |_app, request| {
            let host = request.uri().host().unwrap_or("");
            let path = request.uri().path();
            protocol_service.handle_request(host, path)
        })
        .setup(|app| {
            let app_state = Arc::new(AppState::new());
            app.manage(app_state.clone());

            let rendered_service = Arc::new(Mutex::new(
                RenderedTopicService::new(app.handle().clone())
            ));
            let background_service = Arc::new(Mutex::new(
                BackgroundTopic::new(app.handle().clone())
            ));

            setup_ipc_listeners(app, rendered_service, background_service, app_state)?;

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = initialize_app(app_handle).await {
                    error!("Failed to initialize app: {}", e);
                }
            });
            Ok(())
        })
        .run(context)
        .expect("error while running tauri application");
}

fn main() {
    run();
}
