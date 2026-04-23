# JavaScript Layer

Two webpack-built webview apps, each producing 3 bundles (one per mode).

## Background Webview

Location: `apps/background-webview/`

### Entry Points

| File | Loads | Mode |
|------|-------|------|
| `index-auth.js` | polyfills -> `MainAuth` | Auth |
| `index-import.js` | polyfills -> `MainImport` | Import |
| `index-workspace.js` | polyfills -> `MainWorkspace` | Workspace |

Each entry loads the full polyfill stack in order (11 polyfills), then instantiates the main class.

### IPCHandler (Singleton)

File: `apps/background-webview/src/shared/IPCHandler.js`

```javascript
static getInstance() {
    if (!_instance) { _instance = new IPCHandler(); }
    return _instance;
}
```

Singleton prevents duplicate `listener-ready` signals. On construction:
1. `listen('background-ipc-forward', onMessage)` -- receive from Rust
2. `emit('background-ipc-message', {topic: 'passbolt.background.listener-ready'})` -- signal Rust

Methods: `on(name, cb)`, `once(name, cb)`, `emit(topic, [status], [msg])`, `request(topic, ...args) -> Promise`.

`request()` generates UUID v4 requestId, registers one-shot listener, emits, returns Promise. `_maxConcurrentRequests = 1`, 1s timeout per slot.

### Main Classes

**MainAuth** (`src/main-auth.js`):
- Waits for `passbolt.account.set-current` from Rust
- Clears localStorage (keeps `resourceGridUserSetting-*`)
- Registers: AuthEvents, ConfigEvents, LocaleEvents, DesktopEvents, OrganizationSettingsEvents

**MainImport** (`src/main-import.js`):
- Clears localStorage
- Registers: AuthImportEvents, PownedPasswordEvents

**MainWorkspace** (`src/main-workspace.js`):
- Initializes localStorage + Config
- Registers 25+ event modules (see `listenToEvents()`)
- Starts `StartLoopAuthSessionCheckService` + `KeepSessionAliveService`
- Overrides `navigator.clipboard.writeText` -> IPC clipboard topic
- Emits `BACKGROUND_READY` after all listeners registered

### Polyfill Stack (11 polyfills)

Load order in each `index-*.js`:

1. `browserPolyfill.js` -- `window.chrome = global.browser`
2. `browserSecurityPolyfill.js` -- block keyboard shortcuts + context menu
3. `commandPolyfill.js` -- `chrome.commands` mock
4. `desktopPolyfill.js` -- `chrome.desktop = true`
5. `storagePolyfill.js` -- `chrome.storage.local` via localStorage + batched IPC (50ms debounce)
6. `runtimePolyfill.js` -- `chrome.runtime.id`, `.lastError`, `.getManifest()`
7. `alarmPolyfill.js` -- `chrome.alarms` via setTimeout/setInterval
8. `cookiePolyfill.js` -- `chrome.cookies.get()` via IPC to Rust CookieState
9. `scriptingPolyfill.js` -- wraps browser extension's ScriptingPolyfill
10. `sessionStoragePolyfill.js` -- wraps browser extension's SessionStoragePolyfill
11. `fetchPolyfill.js` -- intercepts `https://` fetch, routes through IPC to Rust HTTP proxy

### Controllers (14)

`desktopAuthenticateController`, `desktopSetAccountController`, `importAccountController`, `verifyPassphraseController`, `authLogoutController`, etc.

### Events (17 modules)

`authEvents`, `clipboardEvents`, `desktopEvents`, `localeEvents`, `rbacEvents`, `themeEvents`, `userEvents`, etc.

## Rendered Webview

Location: `apps/rendered-webview/`

### Entry Points

| File | Loads | Mode |
|------|-------|------|
| `index-auth.js` | polyfills -> `ReactDOM.render(<AppAuthentication>)` | Auth |
| `index-import.js` | polyfills -> `ReactDOM.render(<AppImport>)` | Import |
| `index-workspace.js` | polyfills -> `ReactDOM.render(<AppWorkspace>)` | Workspace |

Each creates `new IPCHandler()` (not singleton) + `new WebviewStorage()`, passes them as props.

### IPCHandler (Per-Instance)

Same API as Background but with `rendered-ipc-forward`/`rendered-ipc-message` event names. Not a singleton -- no risk of duplicate signals on Rendered side.

### WebviewStorage

File: `apps/rendered-webview/src/shared/WebviewStorage.js`

Polyfill for `chrome.storage` expected by the styleguide:

```javascript
storage.local.get(key)     // -> {[key]: JSON.parse(localStorage[key])}
storage.local.set({k: v})  // -> localStorage.setItem + StorageEvent
storage.local.remove(key)
storage.local.clear()
storage.onChanged.addListener(cb)
```

### React Components

**AppAuthentication**: login form. On mount, clears localStorage, emits `rendered.is-ready`, listens for `background.is-ready`. Then fetches site settings, extension version, locale via IPC.

**AppImport**: import wizard. Similar lifecycle.

**AppWorkspace**: full app wrapped in context providers (`ExtAppContextProvider` -> `RbacContextProvider` -> `MfaContextProvider` -> `DialogContextProvider` -> ...). Routes: `/app/passwords/*`, `/app/folders/*`, `/app/users/*`, `/app/groups/*`, `/app/settings`.

**RenderedWebview** (`components/RenderedWebview.js`): listens for `rendered.localstorage-update` and syncs localStorage.

**ResourcesWebviewContext**: watches `window.storage` events for `"resources"` key, navigates to `/app/passwords` on change.

### Polyfill Stack (5 polyfills)

1. `browserPolyfill.js` -- same as Background
2. `browserSecurityPolyfill.js` -- same as Background
3. `fetchPolyfill.js` -- intercepts `passbolt://` fetch, routes through IPC asset-fetch
4. `dragAndDropPolyfill.js` -- mock DataTransfer + synthetic events for WebView2
5. `externalLinkPolyfill.js` -- `window.open()` -> Tauri opener plugin

### Port Propagation

```
index-*.js creates IPCHandler + WebviewStorage
  -> passes as props to <App port={channel} storage={storage}>
    -> App stores in state, passes to ExtAppContextProvider
      -> All child components access port via React context
```

## Dependencies

**Background**: `openpgp@6.1.1`, `passbolt-browser-extension@5.11.1`, `passbolt-styleguide@5.11.3`, `@tauri-apps/api@2.6.0`, `buffer`, `stream-browserify`

**Rendered**: `react@17.0.2`, `react-dom@17.0.2`, `react-router-dom@5.2.0`, `react-i18next@11.8.5`, `passbolt-styleguide@5.11.3`, `@tauri-apps/api@2.6.0`, `@tauri-apps/plugin-opener@2.4.0`

Note: Rendered uses React **17** (not 18), the root `package.json` has React 18 for Vite dev server but the webviews use their own.
