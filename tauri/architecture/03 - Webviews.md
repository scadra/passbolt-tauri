## Two Webviews, Two Roles

The application uses two webviews that run simultaneously:

**Background WebView** -- The hidden layer. It runs the `passbolt-browser-extension` with OpenPGP. It holds the private key, makes all API calls (via Rust), handles encryption and decryption, and manages the session. The user never sees it. It has no visible window and no UI.

**Rendered WebView** -- The visible layer. It runs the React UI (`passbolt-styleguide`). It displays passwords, folders, users, settings. It has no access to the API, the private key, or any credential. Everything it shows comes from the Background through Rust.

### Why two instead of one

A password manager handles two fundamentally different concerns: **displaying sensitive data** and **protecting it**. Putting both in the same context means any vulnerability in the UI (XSS, DOM injection, compromised dependency) gives direct access to the private key and the API session.

By splitting them into two separate webviews, each with its own origin, its own CSP, and its own permissions, a compromise on the visible side cannot reach the sensitive side. Rust sits between them and controls exactly what passes through.

### How they communicate

The two webviews communicate exclusively through IPC (Inter-Process Communication), mediated by Rust. They cannot talk to each other directly and have no way to bypass this principle -- different origins, different CSPs, no shared memory, no shared DOM. All IPC messages are encrypted with AES-GCM (runtime-generated key) via the Tauri isolation pattern before reaching Rust. See [[04 - IPC Layer]] for the full protocol.

### What each webview can and cannot do

| | Rendered (visible) | Background (hidden) |
|-|-------------------|---------------------|
| Display UI | Yes | No |
| Hold private key | No | Yes |
| Make API calls | No | Yes (via Rust proxy) |
| Make HTTP requests | No | No (both go through Rust) |
| Access OS keyring | No | No (Rust handles it) |
| Navigate externally | No (guard blocks it) | No (guard blocks it) |
| Access the other webview | No (different origin) | No (different origin) |

## What is a Webview

Tauri does not ship a browser engine. It delegates rendering to the native webview already present on the OS. This means the application does not control which browser engine runs its code -- it depends on what the user's system provides.

Both webviews use the same native engine on a given machine. The difference is only in what content they load and what they are allowed to do.

## Engines Per Platform

| Platform | Engine | Provided By | Notes |
|----------|--------|------------|-------|
| **macOS** | WKWebView | WebKit (ships with macOS) | Same engine as Safari. Always available, always up to date with the OS. |
| **Windows** | WebView2 | Chromium (via Microsoft Edge) | Requires the Edge WebView2 runtime. Pre-installed on Windows 10/11 with Edge. |
| **Linux** | WebKitGTK | WebKit (via system package) | Must be installed separately (`libwebkit2gtk`). Version depends on the distro. |

The application code (JS, React, OpenPGP) runs identically across engines. The differences are in what the Rust layer can control natively on each platform.

## What Rust Can Control Per Engine

Each engine exposes a different set of native settings. What can be locked down at the OS level varies.

| Setting | macOS (WKWebView) | Windows (WebView2) | Linux (WebKitGTK) |
|---------|-------------------|--------------------|--------------------|
| DevTools | `.devtools()` on builder | `SetAreDevToolsEnabled` | TODO |
| Context menu | Class-level swizzle on `WKContentView` | `SetAreDefaultContextMenusEnabled` | TODO |
| Swipe navigation | `setAllowsBackForwardNavigationGestures` | `SetIsSwipeNavigationEnabled` | TODO |
| Pinch-to-zoom | `setAllowsMagnification` | `SetIsZoomControlEnabled` | TODO |
| Link preview | `setAllowsLinkPreview` | N/A | N/A |
| Password autofill | N/A (WKWebView doesn't offer it) | `SetIsPasswordAutosaveEnabled` | TODO |
| General autofill | N/A | `SetIsGeneralAutofillEnabled` | TODO |
| Host objects (COM) | N/A (Apple concept doesn't exist) | `SetAreHostObjectsAllowed` | N/A |
| Script dialogs | N/A (handled by WKUIDelegate) | `SetAreDefaultScriptDialogsEnabled` | TODO |
| Status bar | N/A | `SetIsStatusBarEnabled` | TODO |
| Browser keys (F12...) | JS polyfill | `SetAreBrowserAcceleratorKeysEnabled` | JS polyfill |
| New window | `on_new_window(Deny)` on builder | `on_new_window(Deny)` on builder | `on_new_window(Deny)` on builder |

**N/A** = the engine doesn't have this feature, no need to disable it.
**TODO** = not yet implemented in the codebase.

## Current Implementation Status

| Platform | Hardening | Native Crates in Cargo.toml |
|----------|-----------|----------------------------|
| **macOS** | Fully implemented | `objc2`, `objc2-web-kit` |
| **Windows** | Stub (referenced in code, not implemented) | None yet |
| **Linux** | Stub (referenced in code, not implemented) | None yet |

The `harden()` function in `webview_security.rs` dispatches to platform-specific functions via `#[cfg]`. macOS calls `apply_macos_settings()` which is fully written. Windows and Linux branches compile but have no implementation yet -- the settings table above shows what needs to be done.

## Why Some Restrictions Need JavaScript

Not every restriction can be enforced at the native level on every platform. Keyboard shortcuts are the main example:

- **macOS**: WKWebView has no API to disable browser keys like Cmd+R (reload) or Cmd+U (view source). These must be caught in JavaScript.
- **Windows**: WebView2 has `SetAreBrowserAcceleratorKeysEnabled` which handles this natively. But the JS polyfill still runs as defense-in-depth.
- **Linux**: WebKitGTK's API for this is not yet explored.

This is why `browserSecurityPolyfill.js` exists in both webviews. It catches keyboard shortcuts and blocks the context menu at the JavaScript level, regardless of platform. It acts as a cross-platform safety net on top of whatever the native layer can do.

## Creation Pattern

Webviews follow a strict creation sequence to avoid any moment where default (insecure) settings are active:

```
1. visible(false)          -- Create the window hidden
2. build()                 -- Build the webview with navigation guards, page load tracking
3. harden()                -- Apply platform-specific restrictions
4. show()                  -- Only then make the window visible (Rendered only)
```

The Background webview is never shown -- it stays hidden with `skip_taskbar(true)`.

This pattern is implemented in `webview_service.rs` and mirrors the original C# implementation's `LoadWebviews()` + `SetWebviewSettings()` flow.

## Platform-Specific Behaviors That Affect the App

### macOS
- WKWebView doesn't have built-in password autofill for custom URL schemes (`passbolt://`), so there's nothing to disable.
- The context menu requires a class-level method swizzle on Apple's private `WKContentView` class because the subview doesn't exist yet at creation time -- it's created lazily when content loads.
- `evaluateJavaScript` is unsafe before page load completes, which is why the `page_loaded` flag exists separately from the `ready` flag.

### Windows (planned)
- WebView2 has COM host objects that allow JavaScript to call native code directly. This must be disabled (`SetAreHostObjectsAllowed(false)`) to prevent privilege escalation.
- The status bar can leak internal `passbolt://` URLs to the user. Must be disabled.
- WebView2's built-in error page would replace the app's content on navigation errors. Must be disabled.

### Linux (planned)
- WebKitGTK's API surface is closer to macOS (both are WebKit-based) but uses GObject instead of Objective-C.
- Package availability varies across distros, which may affect the minimum supported WebKitGTK version.
