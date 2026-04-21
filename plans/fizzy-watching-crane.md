# Fix: macOS main thread deadlock on startup (bilateral handshake race)

## Context

The app freezes on macOS during the auth mode startup. The freeze is a **deadlock on the macOS main thread** caused by a race between a redundant webview navigation and the bilateral handshake's `emit_to` call.

The `webview_service.rs` comment (lines 11-14) already documents that calling `evaluateJavaScript` while a WKWebView is navigating deadlocks the main thread. The `page_loaded` flag was designed to prevent this, but a TOCTOU (Time-of-Check, Time-of-Use) race defeats it.

## Root Cause Trace

```
t=0ms   Both webviews created with index-auth.html URLs (create_webviews)
t=~20ms Background finishes loading, JS sends listener-ready → is-ready
t=26ms  handle_background_ready: rendered NOT ready yet
        → calls handle_navigate_rendered → NAV #1 to same URL (REDUNDANT)
        → WKWebView begins reload asynchronously
t=34ms  Rendered JS had already sent rendered.is-ready (from initial load)
        → handle_rendered_ready fires, both ready → bilateral handshake
        → forward_to_rendered checks is_rv_page_loaded() → TRUE
          (PageLoadEvent::Started from NAV #1 hasn't fired yet — TOCTOU race)
        → spawns async task calling emit_to(rendered, ...)
        → emit_to calls evaluateJavaScript on a navigating WKWebView
        → DEADLOCK
```

The `page_loaded` guard can't protect against this because `navigate()` returns immediately but WKWebView fires `PageLoadEvent::Started` asynchronously — there's a window where the flag is stale.

## Fix

**Move rendered webview navigation out of `handle_background_ready` and into the mode transition handlers.** This eliminates the redundant navigation on initial boot while preserving correct behavior during mode transitions.

### 1. `src-tauri/src/services/topics/background_handlers.rs`

**`handle_background_ready` (line 30-41)**: Remove the `else` branch that navigates rendered. When background is ready but rendered isn't, just do nothing — rendered will send `is-ready` on its own when it finishes loading.

```rust
// BEFORE:
if state.navigation.is_rendered_ready() {
    // bilateral handshake
} else {
    handle_navigate_rendered(app, state);  // ← REMOVE
}

// AFTER:
if state.navigation.is_rendered_ready() {
    // bilateral handshake
}
// (no else — rendered navigates itself or was navigated by the mode transition handler)
```

**`handle_redirect_to_workspace` (line 293-299)**: Add rendered webview navigation (currently only navigates background).

```rust
fn handle_redirect_to_workspace(app: &AppHandle, state: &AppState) {
    state.reset_for_navigation();
    state.navigation.set_mode(AppMode::Workspace);
    let bg_url = build_webview_url("background-webview", &state.navigation.get_background_uuid(), &AppMode::Workspace);
    let rv_url = build_webview_url("rendered-webview", &state.navigation.get_rendered_uuid(), &AppMode::Workspace);
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &bg_url);
    handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &rv_url);  // ← ADD
}
```

**`handle_save_account` (line 75-116)**: In the success branch, also navigate rendered (import → auth transition). The success response to rendered can be removed since the page is navigating away.

```rust
Ok(_) => {
    state.navigation.set_trusted_domain(metadata.domain.clone());
    state.reset_for_navigation();  // ← ADD (reset flags before navigation)
    state.navigation.set_mode(AppMode::Auth);
    let bg_url = build_webview_url("background-webview", &state.navigation.get_background_uuid(), &AppMode::Auth);
    let rv_url = build_webview_url("rendered-webview", &state.navigation.get_rendered_uuid(), &AppMode::Auth);
    handlers::navigate_webview(app, config::BACKGROUND_WEBVIEW_LABEL, &bg_url);
    handlers::navigate_webview(app, config::RENDERED_WEBVIEW_LABEL, &rv_url);  // ← ADD
}
```

**Remove `handle_navigate_rendered`** (line 311-316): No longer needed — dead code after the above changes.

### 2. `src-tauri/src/state/navigation_state.rs`

Add a `handshake_completed` flag as a safety guard against duplicate bilateral handshakes (belt-and-suspenders).

```rust
pub struct NavigationState {
    // ... existing fields ...
    handshake_completed: Mutex<bool>,
}

impl NavigationState {
    pub fn new() -> Self {
        Self {
            // ... existing ...
            handshake_completed: Mutex::new(false),
        }
    }

    pub fn set_handshake_completed(&self, completed: bool) {
        *self.handshake_completed.lock().unwrap() = completed;
    }

    pub fn is_handshake_completed(&self) -> bool {
        *self.handshake_completed.lock().unwrap()
    }

    pub fn reset_ready_flags(&self) {
        // ... existing resets ...
        self.set_handshake_completed(false);  // ← ADD
    }
}
```

### 3. `src-tauri/src/services/topics/rendered_handlers.rs`

Guard the bilateral handshake with `handshake_completed`:

```rust
if state.navigation.is_background_ready() && !state.navigation.is_handshake_completed() {
    state.navigation.set_handshake_completed(true);
    // send bilateral handshake
}
```

### 4. Same guard in `background_handlers.rs::handle_background_ready`

```rust
if state.navigation.is_rendered_ready() && !state.navigation.is_handshake_completed() {
    state.navigation.set_handshake_completed(true);
    // send bilateral handshake
}
```

## Files to modify

| File | Change |
|------|--------|
| `src-tauri/src/services/topics/background_handlers.rs` | Remove navigate-rendered from `handle_background_ready`, add rendered nav to `handle_redirect_to_workspace` and `handle_save_account`, remove `handle_navigate_rendered`, add handshake guard |
| `src-tauri/src/services/topics/rendered_handlers.rs` | Add handshake guard |
| `src-tauri/src/state/navigation_state.rs` | Add `handshake_completed` field + methods, reset in `reset_ready_flags` |

## Verification

1. `cargo test --manifest-path=src-tauri/Cargo.toml` — all Rust tests pass
2. `cargo clippy --manifest-path=src-tauri/Cargo.toml` — no warnings
3. `npm run tauri:dev` — app starts without freeze, auth screen renders
4. Check logs: no redundant NAV on startup, bilateral handshake fires exactly once
5. Manual test mode transitions: import → auth (save account), auth → workspace (login), workspace → auth (logout) — all navigate correctly
