use tauri::{WebviewUrl, WebviewWindowBuilder, Listener};

mod plugins;

/// Initializes the Tauri application with background and rendered webviews.
/// 
/// Creates two webviews:
/// - Background webview: Hidden webview for background operations
/// - Rendered webview: Visible main application window
/// 
/// Sets up event handlers for navigation, window events, and IPC communication.
/// 
/// # Arguments
/// * `app` - Tauri application handle
/// 
/// # Returns
/// * `Result<(), String>` - Success or error message
#[tauri::command]
async fn initialize_app(app: tauri::AppHandle) -> Result<(), String> {
    println!("OnLaunched: Application launched");

    // Background webview (hidden) - Background
    let _background_webview = WebviewWindowBuilder::new(
        &app,
        "background-webview",
        WebviewUrl::App("apps/background-webview".into())
    )
    .title("Passbolt Background")
    .visible(false)
    .resizable(false)
    .build()
    .map_err(|e| format!("Failed to create background webview: {}", e))?;

    // Rendered webview (visible) - styleguide
    let _rendered_webview = WebviewWindowBuilder::new(
        &app,
        "rendered-webview",
        WebviewUrl::App("apps/rendered-webview".into())
    )
    .title("Passbolt Desktop")
    .visible(true)
    .resizable(true)
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .build()
    .map_err(|e| format!("Failed to create rendered webview: {}", e))?;

    /**
    * Listen for IPC messages from webviews
    */
    app.listen("webmessage", |event| {
        println!("WebMessageReceived: {:?}", event.payload());
    });

    /**
    * Listen for new window requests
    */
    app.listen("tauri://new-window", |event| {
        println!("NewWindowRequested: {:?}", event.payload());
    });

    println!("Application initialized successfully");
    Ok(())
}

/// Main entry point for the Tauri application.
/// 
/// Configures and runs the Tauri application with:
/// - Opener plugin for external file/URL handling
/// - Navigation plugin for handling webview navigation events
/// - Automatic application initialization on startup
/// - Event handlers for webview management
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(plugins::navigation::init())
        .invoke_handler(tauri::generate_handler![initialize_app])
        .setup(|app| {
            // Auto-initialize the app
            let app_handle = app.handle().clone();
            
            tauri::async_runtime::spawn(async move {
                if let Err(e) = initialize_app(app_handle).await {
                    eprintln!("Failed to initialize app: {}", e);
                }
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}