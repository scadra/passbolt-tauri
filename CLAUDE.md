# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Passbolt Desktop — a cross-platform password manager built with **Tauri v2** (Rust backend) hosting **two isolated webviews**: a visible React UI (Rendered) and a hidden crypto worker (Background) running the `passbolt-browser-extension` + `openpgp`. They communicate through a topic-based IPC protocol routed by Rust.

## Commands

### Development
```bash
npm run tauri:dev          # Start app with RUST_LOG=debug
npm run tauri:dev:trace    # Start app with RUST_LOG=trace
npm run build:all          # Install deps + build both webviews (required before first tauri:dev)
npm run build:webviews     # Rebuild both webviews without npm install
```

### Rust
```bash
cargo test --manifest-path=src-tauri/Cargo.toml              # All Rust tests
cargo test --manifest-path=src-tauri/Cargo.toml <test_name>  # Single test
cargo clippy --manifest-path=src-tauri/Cargo.toml            # Lint
```

### JavaScript (run from each app directory)
```bash
cd apps/background-webview && npm test        # Background webview tests (node env)
cd apps/rendered-webview && npm test          # Rendered webview tests (jsdom env)
cd apps/background-webview && npm run lint    # ESLint + lockfile-lint
cd apps/rendered-webview && npm run lint
```

## Architecture

### Dual-webview isolation model

```
Rendered WebView (visible)          Background WebView (hidden)
  React 17 UI (passbolt-styleguide)    passbolt-browser-extension + openpgp
  NO API access (CSP blocks it)        ALL API calls happen here
  NO private key access                Private key lives here
          │                                     │
          │  rendered-ipc-message               │  background-ipc-message
          ▼                                     ▼
┌─────────────────────── Rust (Tauri) ───────────────────────┐
│  RenderedTopicService     BackgroundTopic                   │
│  (validates + forwards)   (routes, handles special topics)  │
│                                                             │
│  ProtocolService          CredentialService                 │
│  (passbolt:// handler)    (OS keyring via keyring crate)    │
└─────────────────────────────────────────────────────────────┘
```

### IPC flow
- JS emits via Tauri event system (`@tauri-apps/api/event`), filtered by an isolation iframe (`src/isolation/index.js`) that only allows `emit`/`listen`/`unlisten`
- Rust listens on `rendered-ipc-message` / `background-ipc-message`, routes via `RenderedTopicService` / `BackgroundTopic`
- Rust forwards to the other webview via `emit_to` on `rendered-ipc-forward` / `background-ipc-forward`
- Request/response correlation: the `requestId` becomes the topic of the reply (one-shot consumption via `AppState.proceed_request_id`)

### Content serving
All content served through a custom `passbolt://` protocol handler (`ProtocolService`). Assets embedded in the binary via `rust-embed`. Each webview gets a unique UUID origin (`passbolt://<uuid>.passbolt.local`). Dynamic `index-*.html` generated in-memory with per-mode CSP.

### Topic system
Topics are string constants (`passbolt.<area>.<verb>`) defined via the `define_topics!` macro in `src-tauri/src/topics/*.rs` (33 modules). `AllowedTopics` in `models/messaging.rs` holds 16 native-handled constants. `RenderedTopicService` aggregates all topics into a `HashSet` whitelist at startup.

### State management
`AppState` (`state.rs`): `Arc<Mutex>` fields for `request_ids` (response whitelist), `pending_requests` (response interception for RBAC), `background_uuid`/`rendered_uuid`, and `CredentialCache`. `BackgroundTopic` owns `pending_messages` (buffered while Rendered boots), `_passphrase` (2-hop relay during login), and `rendered_ready` flag.

### Modes
Three modes with distinct bundles: **import** (first launch), **auth** (login), **workspace** (main app). Each mode gets its own CSP and JS bundle pair. Mode transitions happen by navigating webviews to different `passbolt://` paths.

### Credential storage
`CredentialService` uses the `keyring` crate (macOS Keychain / Windows Credential Manager / Linux Secret Service). Two entries: `account-metadata` (JSON) and `account-secret` (armored PGP key). Debug builds write to `~/.local/share/com.passbolt.desktop/debug-credentials/` files instead.

## Key files

| File | Role |
|------|------|
| `src-tauri/src/main.rs` | App bootstrap, webview creation, IPC listener setup |
| `src-tauri/src/services/background_topic_service.rs` | Routes Background→Rendered messages, handles ~15 special topics |
| `src-tauri/src/services/rendered_topic_service.rs` | Validates + forwards Rendered→Background messages |
| `src-tauri/src/services/protocol_service.rs` | `passbolt://` URL routing (domain validation, path dispatch) |
| `src-tauri/src/services/local_folder_service.rs` | Generates index HTML with per-mode CSP, serves embedded assets |
| `src-tauri/src/services/credential_service.rs` | OS keyring CRUD |
| `src-tauri/src/state.rs` | Global thread-safe state |
| `src-tauri/src/models/ipc_model.rs` | IPC wire envelope struct |
| `apps/*/src/shared/IPCHandler.js` | JS-side IPC abstraction (emit/listen/request via Tauri events) |

## Polyfills
Background webview shims WebExtensions APIs (`chrome.storage.local`, `chrome.runtime`, `chrome.alarms`, `chrome.cookies`, etc.) in `apps/background-webview/src/polyfill/`. The storage polyfill triggers cross-webview localStorage sync via IPC. The cookie polyfill delegates to Rust via `BACKGROUND_GET_COOKIE`.
