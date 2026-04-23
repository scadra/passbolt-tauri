## Core Design Decision

The architecture rests on a strict separation between **what the user sees** and **what handles sensitive data**.

Everything security-critical; the OpenPGP private key, encryption/decryption operations, API calls, Credential storage, Session management; is pushed into a **hidden layer** (the Background WebView) that the user has absolutely no access to. 

The visible layer (the Rendered WebView) only displays information it receives through a controlled channel. It cannot reach the API, cannot touch the private key, cannot make HTTP requests on its own.

Between these two layers sits **Rust as a control plane**. It decides what each webview is allowed to do: which messages pass through, which URLs can be navigated to, which domains can be contacted. Nothing moves between the two webviews without Rust validating and routing it.

This design drastically reduces the attack surface. If the visible UI is compromised (XSS, DOM injection), the attacker is stuck in a sandbox with no access to secrets, no network capabilities, and no way to reach the Background. The private key, the API session, the passphrase -- none of it is reachable from the Rendered side.

```
+------------------------------+     +-------------------------------+
|  Rendered WebView (visible)  |     |  Background WebView (hidden)  |
|                              |     |                               |
|  What the user sees          |     |  What the user NEVER sees     |
|  React 17 + styleguide       |     |  passbolt-browser-extension   |
|                              |     |  openpgp                      |
|  NO API access               |     |  ALL API calls (via Rust)     |
|  NO private key              |     |  Private key in memory        |
|  NO direct HTTP              |     |  Encryption / decryption      |
|  NO credential access        |     |  Session management           |
+--------------+---------------+     +---------------+---------------+
               |                                     |
               | rendered-ipc-message                | background-ipc-message
               v                                     v
+------------------------------------------------------------------+
|                    Rust (Tauri v2) -- Control Plane                |
|                                                                    |
|  Controls what each webview can do:                                |
|  - IPC routing + topic validation (whitelist)                      |
|  - Navigation guards (URL restriction per webview)                 |
|  - CSP generation (per webview, per mode)                          |
|  - HTTP proxy (domain restriction, cookie injection)               |
|  - OS keyring (credential storage outside both webviews)           |
|  - Protocol handler (serves assets, no filesystem access)          |
+------------------------------------------------------------------+
```

### Why two webviews specifically

| Concern | Single webview | Dual webview (this architecture) |
|---------|---------------|----------------------------------|
| XSS in the UI | Attacker gets private key, API session, everything | Attacker is sandboxed: no key, no API, no HTTP |
| Private key exposure | Lives in the same JS context as user-facing DOM | Lives in a separate process the user cannot inspect |
| API calls | Originate from the same context that renders untrusted content | Originate from an isolated context with no UI surface |
| Attack surface | Entire application is one target | Two isolated targets, each with minimal permissions |
| CSP tuning | One policy must accommodate both UI needs and crypto needs | Each webview gets a CSP tuned to its exact role |
| Browser extension reuse | Must refactor the extension entirely for desktop | Runs nearly unmodified with polyfills in Background |


