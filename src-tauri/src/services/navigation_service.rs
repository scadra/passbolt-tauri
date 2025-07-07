use tauri::{WebviewUrl, WebviewWindowBuilder, Manager};

#[tauri::command]
async fn initialize_app(app: tauri::AppHandle) -> Result<(), String> {
    println!("OnLaunched: Application launched");

    // Create background webview (hidden)
    let background_webview = WebviewWindowBuilder::new(
        &app,
        "background-webview",
        WebviewUrl::App("apps/background-webview".into())
    )
    .title("Passbolt Background")
    .visible(false)
    .resizable(false)
    .build()
    .map_err(|e| format!("Failed to create background webview: {}", e))?;

    // Create rendered webview (visible)
    let rendered_webview = WebviewWindowBuilder::new(
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

    // Background webview event handlers
    background_webview.on_navigation_handler(|url| {
        println!("BackgroundNavigationStarting: {}", url);
        true // Allow navigation
    });

    let bg_webview_clone = background_webview.clone();
    background_webview.listen("tauri://navigation-completed", move |event| {
        println!("BackgroundNavigationCompleted: {:?}", event.payload());
    });

    background_webview.on_window_event(|event| {
        match event {
            tauri::WindowEvent::WebResourceRequested { .. } => {
                println!("WebResourceRequested: Background webview");
            }
            _ => {}
        }
    });

    background_webview.listen("tauri://webview-created", |event| {
        println!("NewWindowRequested: Background webview - {:?}", event.payload());
    });

    // Rendered webview event handlers  
    rendered_webview.on_navigation_handler(|url| {
        println!("RenderedNavigationStarting: {}", url);
        true // Allow navigation
    });

    let rendered_webview_clone = rendered_webview.clone();
    rendered_webview.listen("tauri://navigation-completed", move |event| {
        println!("RenderedNavigationCompleted: {:?}", event.payload());
    });

    rendered_webview.on_window_event(|event| {
        match event {
            tauri::WindowEvent::WebResourceRequested { .. } => {
                println!("WebResourceRequested: Rendered webview");
            }
            _ => {}
        }
    });

    rendered_webview.listen("tauri://webview-created", |event| {
        println!("NewWindowRequested: Rendered webview - {:?}", event.payload());
    });

    // Listen for IPC messages from webviews
    app.listen("webmessage", |event| {
        println!("WebMessageReceived: {:?}", event.payload());
    });

    println!("Application initialized successfully");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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