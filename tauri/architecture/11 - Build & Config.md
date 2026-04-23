# Build & Config

## Commands

```bash
# Development
npm run tauri:dev              # RUST_LOG=warn,passbolt=info
npm run tauri:dev:debug        # RUST_LOG=debug
npm run tauri:dev:trace        # RUST_LOG=trace

# Build
npm run build:all              # npm install + webpack both webviews (required before first tauri:dev)
npm run build:webviews         # webpack both webviews (no npm install)

# Rust
cargo test --manifest-path=src-tauri/Cargo.toml
cargo clippy --manifest-path=src-tauri/Cargo.toml

# JS tests
cd apps/background-webview && npm test
cd apps/rendered-webview && npm test

# Utility
npm run account:delete:macos   # Delete account from macOS Keychain
```

## Webpack (Webviews)

### Background (`apps/background-webview/webpack.config.js`)

3 entry points -> 3 bundles:
```
index-workspace.js -> dist/background-workspace.js
index-auth.js      -> dist/background-auth.js
index-import.js    -> dist/background-import.js
```

Key config:
- `mode: 'development'`
- `resolve.fallback`: `crypto: false`, `buffer: buffer/`, `stream: stream-browserify`
- `devtool: 'cheap-module-source-map'`
- Plugins: CopyWebpackPlugin (locales), ReplaceInFileWebpackPlugin (locale path rewriting)

### Rendered (`apps/rendered-webview/webpack.config.js`)

3 entry points -> 3 bundles:
```
index-workspace.js -> dist/rendered-workspace.js
index-auth.js      -> dist/rendered-auth.js
index-import.js    -> dist/rendered-import.js
```

Key config:
- `mode: 'development'`
- Babel loader with `@babel/react` preset (JSX support)
- SVGR loader for SVG imports as React components
- Alias: pins `react`, `react-dom`, `react-router-dom`, `react-i18next` to local node_modules
- `devtool: 'inline-source-map'`
- Plugins: CopyWebpackPlugin (locales, themes, fonts), ReplaceInFileWebpackPlugin (path rewriting)

### Build Order

Webviews must be built **before** the Rust binary because `rust-embed` embeds `dist/` at compile time:

```
1. npm install (background-webview)
2. webpack (background-webview)  -> apps/background-webview/dist/
3. npm install (rendered-webview)
4. webpack (rendered-webview)    -> apps/rendered-webview/dist/
5. cargo build                   -> embeds dist/ into binary via rust-embed
```

`npm run build:all` handles steps 1-4.

## Vite (Dev Server Only)

File: `vite.config.js`

```javascript
server: {
    port: 1420,
    strictPort: true,
    hmr: { port: 1421 },
    watch: { ignored: ["**/src-tauri/**"] },
}
```

Referenced by `tauri.conf.json` `build.devUrl`. Not used for production webview builds.

## tauri.conf.json

File: `src-tauri/tauri.conf.json`

```json
{
    "productName": "passbolt-desktops",
    "version": "0.1.0",
    "identifier": "com.passbolt-desktops.app",
    "build": {
        "beforeDevCommand": "npm run dev",
        "devUrl": "http://localhost:1420",
        "beforeBuildCommand": "npm run build:all",
        "frontendDist": "../apps/rendered-webview/dist"
    },
    "app": {
        "security": {
            "pattern": { "use": "isolation", "options": { "dir": "../src/isolation" } },
            "csp": null
        }
    },
    "bundle": { "active": true, "targets": "all" }
}
```

- `csp: null` -- CSP is generated dynamically by `local_folder_service`
- `pattern: isolation` -- enables the [[09 - Security Layers#1. Isolation Iframe|isolation iframe]]
- `frontendDist` -- points to Rendered dist (but actual serving is via protocol handler)

## Cargo Release Profile

File: `src-tauri/Cargo.toml`

```toml
[profile.release]
panic = "abort"       # No unwinding
codegen-units = 1     # Max optimization
lto = true            # Link-time optimization
opt-level = "s"       # Optimize for size
strip = true          # Strip debug symbols
```

## VS Code

File: `.vscode/extensions.json` -- recommends tauri-vscode, rust-analyzer, vscode-lldb
File: `.vscode/launch.json` -- LLDB debug config for `passbolt` binary with `RUST_LOG=debug`

## Capabilities

File: `src-tauri/capabilities/default.json`

3 windows (`main`, `rendered-webview`, `background-webview`) with permissions:
- `core:default`, `opener:default`
- `core:event:allow-listen`, `core:event:allow-emit` for both webviews
- `opener:allow-open-url` restricted to `help.passbolt.com`, `www.passbolt.com`, `en.wikipedia.org/wiki/Phishing`
