# State

All application state lives in `AppState`, a Tauri-managed struct. Accessed via `app.state::<AppState>()` from any event handler.

## Root

File: `state/app_state.rs`

```rust
pub struct AppState {
    pub ipc: IpcState,
    pub navigation: NavigationState,
    pub auth: AuthState,
    pub cookies: CookieState,
    pub credentials: CredentialCache,
}
```

`reset_for_navigation()` resets ready flags + clears pending messages (called on mode transitions).

## IpcState

File: `state/ipc_state.rs`

```rust
pub struct IpcState {
    request_ids: Mutex<HashSet<String>>,              // One-shot requestId whitelist
    pending_requests: Mutex<HashMap<String, String>>,  // requestId -> original topic (for RBAC)
    pending_messages: Mutex<Vec<IpcMessage>>,           // Buffer for Rendered during boot
}
```

- `add_request_id(id)` / `proceed_request_id(id)` -- register / consume (one-shot)
- `push_pending_message(msg)` / `drain_pending_messages()` -- buffer / drain cycle
- `add_pending_request(id, topic)` / `take_pending_request_by_topic(id)` -- RBAC tracking

Three separate Mutexes: these fields are accessed independently at different frequencies.

## NavigationState

File: `state/navigation_state.rs`

```rust
pub struct NavigationState {
    background_ready: Mutex<bool>,
    rendered_ready: Mutex<bool>,
    background_page_loaded: Mutex<bool>,
    rendered_page_loaded: Mutex<bool>,
    current_mode: Mutex<AppMode>,
    trusted_domain: Mutex<Option<String>>,
    isolation_schema: Mutex<Option<String>>,
    background_uuid: Mutex<String>,
    rendered_uuid: Mutex<String>,
    mfa_active: Mutex<bool>,
}
```

Individual Mutex per field -- no two fields need atomic update together.

Important distinction:
- `*_page_loaded` = WKWebView finished loading HTML (safe for `evaluateJavaScript` / `emit_to`)
- `*_ready` = JS app has mounted its listeners (safe for IPC messages)

UUIDs are generated once at `NavigationState::new()` via `uuid::Uuid::new_v4()`.

## AuthState

File: `state/auth_state.rs`

```rust
pub struct AuthState {
    passphrase: Mutex<Option<String>>,
}
```

- `set_passphrase()` -- cache during login
- `take_passphrase()` -- consume one-shot during workspace boot

Never persisted. Exists only to bridge the passphrase across mode transitions (Auth -> Workspace) when both webviews reload.

## CookieState

File: `state/cookie_state.rs`

```rust
pub struct CookieState {
    cookies: Arc<Mutex<HashMap<String, String>>>,
}
```

`Arc<Mutex>` because it's cloned into async tasks spawned by `handle_http_fetch()`. The `reqwest` client runs in a separate tokio task.

Methods: `set_cookie()`, `get_cookie()`, `get_all_cookies()`, `to_cookie_header()`.

## CredentialCache

File: `state/credential_cache.rs`

```rust
pub struct CredentialCache {
    inner: Mutex<Inner>,
}

struct Inner {
    metadata: Option<AccountMetadata>,
    secret: Option<AccountSecret>,  // ZeroizeOnDrop
}
```

**Single Mutex** covering both fields so `clear()` is atomic -- no window where metadata exists but secret doesn't.

`AccountSecret` derives `ZeroizeOnDrop`: private key bytes are overwritten with zeros on drop.

## Mutex Strategy

| Pattern | When | Example |
|---------|------|---------|
| Individual Mutex per field | Accessed independently | NavigationState (10 fields, 10 Mutexes) |
| Single Mutex for group | Must be consistent together | CredentialCache (metadata + secret) |
| Arc<Mutex> | Shared across async tasks | CookieState (cloned into reqwest tasks) |

**No nested locks**: handlers never hold two Mutexes simultaneously. This prevents deadlocks by design.

## Thread Safety

Tauri event handlers run on the main thread. Long-running operations (`emit_to`, HTTP fetch) are spawned via `tauri::async_runtime::spawn()`. Spawned tasks receive clones of `Arc` data or resolved values, not Mutex guards.
