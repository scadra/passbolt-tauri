use tauri::{WebviewUrl, WebviewWindowBuilder, Listener};
use uuid::Uuid;

mod plugins;
mod services;

use services::local_folder_service::LocalFolderService;

#[tauri::command]
async fn initialize_app(app: tauri::AppHandle) -> Result<(), String> {
    println!("OnLaunched: Application launched");

    let background_uuid = Uuid::new_v4().to_string();
    let rendered_uuid = Uuid::new_v4().to_string();

    println!("🔒 Background domain: passbolt://{}.passbolt.local", background_uuid);
    println!("🔒 Rendered domain: passbolt://{}.passbolt.local", rendered_uuid);

    // Background webview avec protocole personnalisé (non-external)
    let _background_webview = WebviewWindowBuilder::new(
        &app,
        "background-webview",
        WebviewUrl::CustomProtocol(
            format!("passbolt://{}.passbolt.local/background", background_uuid)
                .parse()
                .map_err(|e| format!("Invalid background URL: {}", e))?
        )
    )
    .title("Passbolt Background")
    .visible(false)
    .resizable(false)
    .build()
    .map_err(|e| format!("Failed to create background webview: {}", e))?;

    // Rendered webview avec protocole personnalisé (non-external)
    let _rendered_webview = WebviewWindowBuilder::new(
        &app,
        "rendered-webview",
        WebviewUrl::CustomProtocol(
            format!("passbolt://{}.passbolt.local/app", rendered_uuid)
                .parse()
                .map_err(|e| format!("Invalid rendered URL: {}", e))?
        )
    )
    .title("Passbolt Desktop")
    .visible(true)
    .resizable(true)
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .build()
    .map_err(|e| format!("Failed to create rendered webview: {}", e))?;

    app.listen("webmessage", |event| {
        println!("WebMessageReceived: {:?}", event.payload());
    });

    app.listen("tauri://new-window", |event| {
        println!("NewWindowRequested: {:?}", event.payload());
    });

    println!("✅ Application initialized successfully");
    Ok(())
}

pub fn run() {
    let service = LocalFolderService::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(plugins::navigation::init())
        .register_uri_scheme_protocol("passbolt", move |_app, request| {
            let host = request.uri().host().unwrap_or("");
            let path = request.uri().path();
            
            println!("📁 Request: passbolt://{}{}", host, path);
            
            //passbolt.local 
            if !host.ends_with(".passbolt.local") {
                println!("❌ Unauthorized domain: {}", host);
                return tauri::http::Response::builder()
                    .status(403)
                    .body(b"Unauthorized domain".to_vec())
                    .unwrap();
            }
            
            let uuid = host.replace(".passbolt.local", "");
            
            let result = match path {
                "/background" => {
                    println!("🔒 Serving background app for UUID: {}", uuid);
                    service.create_background_index("background-workspace", None, &uuid)
                },
                "/app" => {
                    println!("🔐 Serving rendered app for UUID: {}", uuid);
                    service.create_rendered_index("rendered-workspace", "default", None, &uuid)
                },
                // Servir vos assets webview
                path if path.starts_with("/apps/") => {
                    println!("📄 Serving webview asset: {}", path);
                    service.serve_file(path)
                },
                _ => {
                    println!("❌ Not found: {}", path);
                    tauri::http::Response::builder()
                        .status(404)
                        .body(b"Page not found".to_vec())
                        .map_err(|e| e.into())
                }
            };
            
            result.unwrap_or_else(|e| {
                println!("💥 Error serving content: {}", e);
                tauri::http::Response::builder()
                    .status(500)
                    .body(b"Internal server error".to_vec())
                    .unwrap()
            })
        })
        .invoke_handler(tauri::generate_handler![initialize_app])
        .setup(|app| {
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