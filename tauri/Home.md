# Passbolt Desktop

Cross-platform password manager built with **Tauri v2** (Rust) + **two isolated webviews** (React UI + headless crypto worker).

## Architecture
1. [[01 - Overview]] - Design decision, dual-webview model, why this architecture
2. [[02 - Structure]] - Project layout: Rust backend, Background webview, Rendered webview
3. [[03 - Webviews]] - Two webviews: what they do, why, platform engines, hardening per OS
4. [[04 - IPC Layer]] - How the two webviews communicate through Rust: events, wire format, forwarding
5. [[05 - Topic System]] - 33 topic modules, `define_topics!` macro, `AllowedTopics` whitelist
6. [[06 - Routing]] - How `RenderedTopicService` and `BackgroundTopicService` dispatch messages
7. [[07 - Protocol Handler]] - `passbolt://` custom scheme, `rust-embed` assets, dynamic HTML
8. [[08 - State]] - `AppState` decomposition: 5 substates, mutex strategy, thread safety
9. [[09 - Security Layers]] - CSP generation, isolation iframe, navigation guards, platform hardening
10. [[10 - JavaScript Layer]] - IPCHandler, WebviewStorage, polyfill stack, entry points
11. [[11 - Build & Config]] - Webpack, Cargo, tauri.conf.json, capabilities

## Security
- [[01- Isolation Pattern]] - Sandboxed iframe between frontend and Rust, AES-GCM encryption, command filtering
