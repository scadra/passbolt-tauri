# Security Layers

Defense-in-depth with overlapping layers. Each layer blocks a different class of attack.

## Layer Summary

```
1. Isolation Iframe    -- Only event emit/listen/unlisten pass through to Rust
2. CSP per webview     -- No inline scripts, restricted connect-src
3. Origin isolation    -- Unique UUID per webview per session
4. Topic whitelist     -- AllowedTopics validates every Rendered message
5. RequestId validation-- UUID regex on all requestIds
6. Navigation guards   -- Only own origin (+ MFA exception on Rendered)
7. Platform hardening  -- Disable DevTools, zoom, shortcuts, context menu
8. HTTP proxy          -- Domain whitelist for all outbound requests
9. RBAC injection      -- Desktop-specific permission denials
10. Credential zeroize -- AccountSecret scrubbed on drop
```

## 1. Isolation Iframe

File: `src/isolation/index.js` (22 lines)

```javascript
window.__TAURI_ISOLATION_HOOK__ = (payload) => {
    const allowedCommands = [
        'plugin:event|emit',
        'plugin:event|listen',
        'plugin:event|unlisten',
    ];
    if (!allowedCommands.includes(payload.cmd)) {
        return null;  // Blocked
    }
    return payload;   // Allowed
};
```

Enabled in `tauri.conf.json`:
```json
{ "app": { "security": { "pattern": { "use": "isolation", "options": { "dir": "../src/isolation" } } } } }
```

All other Tauri commands (invoke, shell, fs, http, dialog, process) are blocked. Even if a webview is XSS'd, the attacker can only emit/listen events.

## 2. CSP Generation

File: `services/local_folder_service.rs`

CSP is `null` in `tauri.conf.json` -- generated dynamically per (webview x mode) and injected as `<meta>` tag.

### Rendered CSP Matrix

| Directive | Import | Auth | Workspace |
|-----------|--------|------|-----------|
| `default-src` | `'none'` | `'none'` | `'none'` |
| `form-action` | `'none'` | `'none'` | `'none'` |
| `frame-src` | `{isolation}://localhost` | same | same |
| `script-src` | `passbolt://{rv_uuid}...` | same | same |
| `style-src` | `'self' 'unsafe-inline'` + themes | same | same |
| `font-src` | own origin `/fonts/` | same | same |
| `img-src` | own origin `/img/` | + trusted_domain | + trusted_domain + `data:` + `blob:` |
| `connect-src` | IPC only | IPC + self | IPC + self |

### Background CSP (all modes)

```
default-src 'none'; form-action 'none';
frame-src {isolation}://localhost;
script-src passbolt://{bg_uuid}.passbolt.local/;
style-src 'unsafe-inline';
connect-src ipc://localhost/plugin:event|listen ipc://localhost/plugin:event|emit
            passbolt://{bg_uuid}.passbolt.local/;
```

No `https:` in `connect-src` -- all HTTP goes through Rust.

## 3. Origin Isolation

Each webview gets a UUID v4 subdomain at startup:

```
Rendered:   passbolt://a1b2c3d4.passbolt.local
Background: passbolt://e5f6g7h8.passbolt.local
```

Same-origin policy prevents cross-webview access. UUIDs change every session.

## 4-5. Topic Whitelist + RequestId Validation

See [[06 - Routing]]. `RenderedTopicService` checks topic against `AllowedTopics` HashSet and requestId against UUID regex. Invalid messages are dropped.

## 6. Navigation Guards

File: `services/navigation_service.rs`

**Rendered guard**: allows own origin + MFA URLs on trusted domain when `mfa_active = true`. Session expiry (`/auth/login`) triggers logout. Everything else blocked.

**Background guard**: own origin only, no exceptions.

**New window**: `on_new_window(Deny)` on both webviews -- no `window.open()`, no `target="_blank"`.

**Page load**: `on_page_load` tracks `*_page_loaded` flags. On Rendered, also detects MFA completion (`has_mfa_completed()` check at `webview_service.rs:83`).

## 7. Platform Hardening

File: `services/webview_security.rs`

### macOS (WKWebView, via objc2)
- `setAllowsBackForwardNavigationGestures(false)` -- no swipe-back
- `setAllowsMagnification(false)` -- no pinch-to-zoom
- `setAllowsLinkPreview(false)` -- no force-touch preview
- Context menu disabled via class-level method swizzle on private `WKContentView` (one-time via `Once`)

### Windows (WebView2, via webview2-com)
- DevTools disabled (release)
- Password autosave, general autofill, host objects, script dialogs disabled
- Context menu, status bar, built-in error page, zoom disabled (release)
- Browser accelerator keys (F12, Ctrl+F, Ctrl+U), swipe nav, pinch zoom disabled

### JS Layer (both webviews)
File: `apps/*/src/polyfill/browserSecurityPolyfill.js`

Capture-phase `keydown` listener blocks: F3, F5, F10, Cmd/Ctrl+R, +U, +F, +G, +P, +O, +J.
Allows: Cmd/Ctrl+C/V/X (clipboard), +A (select all), +Z (undo), Tab (focus).
Capture-phase `contextmenu` listener blocks native right-click menu.

## 8. HTTP Proxy

File: `services/http_fetch_service.rs`

Background fetch polyfill routes HTTPS through IPC to Rust's `reqwest` client.
Domain whitelist: only `trusted_domain` + `https://api.pwnedpasswords.com`.
Cookie injection from `CookieState`, Set-Cookie harvesting back.

## 9. RBAC Injection

File: `services/rbac_service.rs`

Desktop-denied actions forced to `Deny`: `Administration.viewWorkspace`, `Duo.configuration`, `Avatar.upload`, `Mobile.transfer`, `Desktop.transfer`, `Profil.accountRecovery`.

Injected in `handle_response_forward()` when the response matches a pending RBAC request.

## 10. Credential Zeroize

`AccountSecret` derives `ZeroizeOnDrop`. `CredentialCache.clear()` is atomic (single Mutex). Passphrase in `AuthState` is consumed one-shot via `take_passphrase()`.

## Capabilities

File: `src-tauri/capabilities/default.json`

```json
{
    "windows": ["main", "rendered-webview", "background-webview"],
    "permissions": [
        "opener:default", "core:default",
        "core:event:allow-listen",
        "core:event:allow-emit",
        { "identifier": "opener:allow-open-url",
          "allow": [
            { "url": "https://help.passbolt.com/*" },
            { "url": "https://www.passbolt.com/*" },
            { "url": "https://en.wikipedia.org/wiki/Phishing" }
          ]
        }
    ]
}
```

Only event emit/listen + URL opener (3 whitelisted domains).
