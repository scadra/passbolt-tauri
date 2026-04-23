# Structure

How the project is organized on disk. The codebase has three main areas: the Rust backend that acts as the control plane, the Background webview that handles all sensitive operations, and the Rendered webview that displays the UI.

## Root

```
passbolt-desktops/
├── package.json                            # Build scripts
├── vite.config.js                          # Dev server only
├── src/
│   └── isolation/
│       └── index.js                        # Tauri isolation hook
├── src-tauri/                              # Rust backend
└── apps/
    ├── background-webview/                 # Hidden crypto worker
    └── rendered-webview/                   # Visible React UI
```

The `isolation/` folder contains a single JS file that acts as a security gate between the webviews and the Tauri API. Everything the webviews want to do must pass through it.

## Rust Backend (`src-tauri/`)

```
src-tauri/
├── Cargo.toml                              # Dependencies, release profile
├── tauri.conf.json                         # Tauri config
├── build.rs                                # Build script
├── capabilities/
│   └── default.json                        # Permissions
└── src/
    ├── main.rs                             # App startup
    ├── lib.rs                              # Crate root, macros
    ├── config.rs                           # String constants
    ├── error.rs                            # Error types
    ├── state/                              # Application state
    │   ├── app_mode.rs                     # Import / Auth / Workspace modes
    │   ├── app_state.rs                    # Groups all substates
    │   ├── ipc_state.rs                    # Request tracking, message buffer
    │   ├── navigation_state.rs             # Ready flags, UUIDs, trusted domain
    │   ├── auth_state.rs                   # Passphrase relay
    │   ├── cookie_state.rs                 # Session cookies
    │   └── credential_cache.rs             # Cached credentials from keyring
    ├── models/                             # Data structures
    │   ├── ipc_message.rs                  # Message envelope
    │   ├── messaging.rs                    # Topic whitelist
    │   ├── account_metadata.rs             # Public account info
    │   └── account_secret.rs               # Private key (zeroized on drop)
    ├── services/                           # Business logic
    │   ├── ipc_listener_service.rs         # Registers event listeners
    │   ├── rendered_topic_service.rs       # Routes messages from Rendered
    │   ├── background_topic_service.rs     # Routes messages from Background
    │   ├── protocol_service.rs             # Serves passbolt:// URLs
    │   ├── local_folder_service.rs         # Generates HTML and CSP per mode
    │   ├── webview_service.rs              # Creates and configures webviews
    │   ├── webview_security.rs             # Platform hardening
    │   ├── navigation_service.rs           # URL validation guards
    │   ├── keyring_service.rs              # OS keyring access
    │   ├── http_fetch_service.rs           # HTTP proxy for Background
    │   ├── rbac_service.rs                 # Permission overrides for desktop
    │   ├── mfa_service.rs                  # MFA URL helpers
    │   └── topics/                         # Message handlers
    │       ├── handlers.rs                 # Shared forwarding logic
    │       ├── background_handlers.rs      # Handles Background-specific topics
    │       └── rendered_handlers.rs        # Handles Rendered-specific topics
    └── topics/                             # All supported topics
```

`state/` holds all the shared application state, what mode the app is in, which webviews are ready, cached credentials, session cookies. Each concern is isolated in its own file.

`models/` defines the data structures that travel through the system: the IPC message envelope, the topic whitelist, and the account data stored in the OS keyring.

`services/` is where the business logic lives. Each service handles one responsibility: routing IPC messages, serving assets via the custom protocol, managing the OS keyring, proxying HTTP requests, hardening webviews per platform. The `topics/` subfolder inside services contains the actual message handlers, the code that runs when a specific topic arrives.

`topics/` (at the root of `src/`) is separate from `services/topics/`. It only defines string constants , the names of all valid IPC topics. These constants are used by the services to validate and match incoming messages.

## Background WebView (`apps/background-webview/`)

Hidden crypto worker. Runs the passbolt-browser-extension with polyfills.

```
background-webview/
├── package.json
├── webpack.config.js
├── index-auth.js                           # Auth mode entry point
├── index-import.js                         # Import mode entry point
├── index-workspace.js                      # Workspace mode entry point
├── dist/                                   # Build output (embedded in binary)
└── src/
    ├── main-auth.js                        # Auth mode logic
    ├── main-import.js                      # Import mode logic
    ├── main-workspace.js                   # Workspace mode logic
    ├── shared/
    │   └── IPCHandler.js                   # IPC communication with Rust
    ├── polyfill/                           # WebExtension API shims
    │   ├── browserPolyfill.js              # Browser namespace alias
    │   ├── browserSecurityPolyfill.js      # Keyboard/context menu blocking
    │   ├── storagePolyfill.js              # chrome.storage replacement
    │   ├── fetchPolyfill.js                # Routes HTTPS through Rust
    │   ├── cookiePolyfill.js               # Cookie access via Rust
    │   ├── alarmPolyfill.js                # Timer-based alarms
    │   ├── runtimePolyfill.js              # Extension runtime mock
    │   ├── commandPolyfill.js              # Extension commands mock
    │   ├── desktopPolyfill.js              # Desktop detection flag
    │   ├── scriptingPolyfill.js            # Script injection mock
    │   └── sessionStoragePolyfill.js
    ├── controllers/                        # Action handlers
    │   ├── desktopAuthenticateController.js
    │   ├── desktopSetAccountController.js
    │   ├── importAccountController.js
    │   └── ...
    ├── events/                             # IPC event listeners
    │   ├── authEvents.js
    │   ├── clipboardEvents.js
    │   ├── desktopEvents.js
    │   └── ...
    └── services/                           # Background services
        ├── alarm/
        │   ├── authSessionCheckService.js
        │   └── globalAlarmService.js
        └── loginUserService.js
```

`polyfill/` bridges the gap between WebExtension APIs (what the browser extension expects) and the Tauri environment (what actually exists). Each polyfill replaces one browser API with an equivalent that works through IPC.

`controllers/` handle specific user actions (login, import, passphrase verification). `events/` register IPC listeners for each domain (auth, clipboard, theme). `services/` run background tasks like session checking and keepalive.

## Rendered WebView (`apps/rendered-webview/`)

Visible React UI. Only displays data it receives from Background through IPC. Has no direct access to the API, the private key, or any sensitive data.

```
rendered-webview/
├── package.json
├── webpack.config.js
├── index-auth.js                           # Auth mode entry point
├── index-import.js                         # Import mode entry point
├── index-workspace.js                      # Workspace mode entry point
├── dist/                                   # Build output (embedded in binary)
└── src/
    ├── AppAuthentication.js                # Login screen
    ├── AppImport.js                        # Import wizard
    ├── AppWorkspace.js                     # Main app with routing
    ├── shared/
    │   ├── IPCHandler.js                   # IPC communication with Rust
    │   └── WebviewStorage.js               # Storage abstraction
    ├── polyfill/                           # Platform compatibility shims
    │   ├── browserPolyfill.js              # Browser namespace alias
    │   ├── browserSecurityPolyfill.js      # Keyboard/context menu blocking
    │   ├── fetchPolyfill.js                # Loads embedded assets via IPC
    │   ├── dragAndDropPolyfill.js          # WebView drag-and-drop fix
    │   └── externalLinkPolyfill.js         # Opens links in system browser
    ├── components/
    │   ├── RenderedWebview.js              # Syncs data from Background
    │   ├── OrchestrateLoginBoxMain.js
    │   └── Stylesheet.js
    └── contexts/
        └── ResourcesWebviewContext.js
```

`polyfill/` here is lighter than Background's -- it only handles asset loading, drag-and-drop compatibility, and external link opening. No need to shim WebExtension APIs since the Rendered side doesn't run the browser extension.

`components/` contains React components that bridge IPC data to the UI. `contexts/` provides React context that watches for data changes and triggers navigation.
