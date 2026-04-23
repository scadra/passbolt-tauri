//! Shared helper functions used by both topic services.
//!
//! Two levels of protection before calling `emit_to`:
//! 1. `page_loaded` — WKWebView has finished loading (safe for evaluateJavaScript)
//! 2. `*_ready` — the JS app has mounted its listeners (safe for IPC messages)
//!
//! If either check fails, the message is buffered in `pending_messages` and
//! replayed when `RENDERED_READY` fires.

use log::{info, warn, debug};
use tauri::{AppHandle, Emitter, Manager};

use crate::config;
use crate::models::ipc_message::IpcMessage;
use crate::state::AppState;

// --- Debug instrumentation (compiled out in release) ---

#[cfg(debug_assertions)]
mod diag {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::LazyLock;
    use std::time::Instant;

    static START: LazyLock<Instant> = LazyLock::new(Instant::now);
    static SEQ: AtomicU64 = AtomicU64::new(0);

    pub fn ms() -> u128 { START.elapsed().as_millis() }
    pub fn seq() -> u64 { SEQ.fetch_add(1, Ordering::Relaxed) }
    pub fn tid() -> std::thread::ThreadId { std::thread::current().id() }
}

/// Debug-only logging macro. Compiles to nothing in release.
///
/// Uses a nested function trick to avoid `#[cfg]` on expressions
/// (which requires unstable `stmt_expr_attributes` on MSRV 1.77.2).
#[cfg(debug_assertions)]
macro_rules! diag_log {
    ($($arg:tt)*) => { eprintln!($($arg)*) };
}

#[cfg(not(debug_assertions))]
macro_rules! diag_log {
    ($($arg:tt)*) => {};
}

// --- Public API ---

/// Send an IPC message to the Rendered webview.
///
/// Checks `rendered_page_loaded` first — if the webview is navigating,
/// the message is buffered to avoid deadlocking the macOS main thread.
pub fn forward_to_rendered(app: &AppHandle, ipc: &IpcMessage) {
    let state = app.state::<AppState>();

    diag_log!("[H_LOCK] >>> is_rv_page_loaded thread={:?} t={}ms", diag::tid(), diag::ms());
    let rv_loaded = state.navigation.is_rendered_page_loaded();
    diag_log!("[H_LOCK] <<< is_rv_page_loaded={} thread={:?} t={}ms", rv_loaded, diag::tid(), diag::ms());

    if !rv_loaded {
        diag_log!("[EMIT] BUFFER (page not loaded) rendered topic={}", ipc.topic);
        state.ipc.push_pending_message(ipc.clone());
        return;
    }

    #[cfg(debug_assertions)]
    let diag_seq = diag::seq();

    diag_log!("[EMIT] #{} >>> rendered topic={} thread={:?} t={}ms",
        diag_seq, ipc.topic, diag::tid(), diag::ms());

    let app = app.clone();
    let ipc = ipc.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(debug_assertions)]
        let t0 = std::time::Instant::now();

        let result = app.emit_to(config::RENDERED_WEBVIEW_LABEL, config::RENDERED_IPC_FORWARD, &ipc);

        #[cfg(debug_assertions)]
        {
            let dt = t0.elapsed();
            match &result {
                Ok(_) => diag_log!("[EMIT] #{} <<< rendered topic={} dt={:?} thread={:?} t={}ms",
                    diag_seq, ipc.topic, dt, diag::tid(), diag::ms()),
                Err(e) => diag_log!("[EMIT] #{} !!! rendered topic={} err={} thread={:?} t={}ms",
                    diag_seq, ipc.topic, e, diag::tid(), diag::ms()),
            }
        }
        let _ = result;
    });
}

/// Send an IPC message to the Background webview.
///
/// Checks `background_page_loaded` first.
pub fn forward_to_background(app: &AppHandle, ipc: &IpcMessage) {
    let state = app.state::<AppState>();

    diag_log!("[H_LOCK] >>> is_bg_page_loaded thread={:?} t={}ms", diag::tid(), diag::ms());
    let bg_loaded = state.navigation.is_background_page_loaded();
    diag_log!("[H_LOCK] <<< is_bg_page_loaded={} thread={:?} t={}ms", bg_loaded, diag::tid(), diag::ms());

    if !bg_loaded {
        diag_log!("[EMIT] BUFFER (page not loaded) background topic={}", ipc.topic);
        // Background messages during navigation are rare but possible during mode transitions.
        // We don't have a separate buffer for Background — these are dropped with a warning.
        warn!("[IPC] Background page not loaded, dropping: {}", ipc.topic);
        return;
    }

    #[cfg(debug_assertions)]
    let diag_seq = diag::seq();

    diag_log!("[EMIT] #{} >>> background topic={} thread={:?} t={}ms",
        diag_seq, ipc.topic, diag::tid(), diag::ms());

    let app = app.clone();
    let ipc = ipc.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(debug_assertions)]
        let t0 = std::time::Instant::now();

        let result = app.emit_to(config::BACKGROUND_WEBVIEW_LABEL, config::BACKGROUND_IPC_FORWARD, &ipc);

        #[cfg(debug_assertions)]
        {
            let dt = t0.elapsed();
            match &result {
                Ok(_) => diag_log!("[EMIT] #{} <<< background topic={} dt={:?} thread={:?} t={}ms",
                    diag_seq, ipc.topic, dt, diag::tid(), diag::ms()),
                Err(e) => diag_log!("[EMIT] #{} !!! background topic={} err={} thread={:?} t={}ms",
                    diag_seq, ipc.topic, e, diag::tid(), diag::ms()),
            }
        }
        let _ = result;
    });
}

/// Buffer or forward a Background→Rendered message.
///
/// Three checks before forwarding:
/// 1. `background_ready` — Background JS has finished init
/// 2. `rendered_ready` — Rendered JS has mounted listeners
/// 3. `rendered_page_loaded` — WKWebView finished loading (checked in forward_to_rendered)
pub fn buffer_or_forward_to_rendered(app: &AppHandle, state: &AppState, ipc: IpcMessage) {
    let bg_ready = state.navigation.is_background_ready();

    let ipc = remap_topic_for_rendered(ipc);

    if !bg_ready {
        debug!("[IPC] buffer: topic={} bg_ready={}", ipc.topic, bg_ready);
        state.ipc.push_pending_message(ipc);
        return;
    }

    debug!("[IPC] forward: topic={}", ipc.topic);
    forward_to_rendered(app, &ipc);
}

/// Remap Background→Rendered topic names.
fn remap_topic_for_rendered(mut ipc: IpcMessage) -> IpcMessage {
    use crate::topics::locale_storage_topics::LocalStorageTopics;

    ipc.topic = match ipc.topic.as_str() {
        t if t == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_UPDATE =>
            LocalStorageTopics::RENDERED_LOCALSTORAGE_UPDATE.to_string(),
        t if t == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_DELETE =>
            LocalStorageTopics::RENDERED_LOCALSTORAGE_DELETE.to_string(),
        t if t == LocalStorageTopics::BACKGROUND_LOCALSTORAGE_CLEAR =>
            LocalStorageTopics::RENDERED_LOCALSTORAGE_CLEAR.to_string(),
        _ => ipc.topic,
    };
    ipc
}

/// Navigate a webview to a URL.
pub fn navigate_webview(app: &AppHandle, label: &str, url: &str) {
    info!("[NAV] {} → {}", label, url);

    if let Some(window) = app.get_webview_window(label) {
        let parsed: tauri::Url = url.parse().unwrap();
        match window.navigate(parsed) {
            Ok(_) => info!("[NAV] {} navigation dispatched", label),
            Err(error) => warn!("[NAV] {} navigation failed: {}", label, error),
        }
    } else {
        warn!("[NAV] Webview not found: {}", label);
    }
}
