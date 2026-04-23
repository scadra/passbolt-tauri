# Protocol Handler

Custom `passbolt://` URL scheme. Assets embedded in the binary via `rust-embed`. Dynamic HTML generated per mode with CSP.

## Registration

File: `main.rs:28-33`

```rust
.register_asynchronous_uri_scheme_protocol(config::PROTOCOL_SCHEME, move |ctx, request, responder| {
    let url = request.uri().to_string();
    let state = ctx.app_handle().state::<AppState>();
    let response = protocol_service.handle_request(&url, &state);
    responder.respond(response);
})
```

`ProtocolService` is created once at startup with the isolation schema, then captured by the closure.

## URL Format

```
passbolt://{uuid}.passbolt.local/apps/{webview}/{path}
```

- `{uuid}` -- random UUID v4, unique per webview per session
- `.passbolt.local` -- non-routable domain suffix
- `{webview}` -- `rendered-webview` or `background-webview`

## Embedded Assets

File: `services/protocol_service.rs:14-28`

```rust
#[derive(RustEmbed)]
#[folder = "../apps/rendered-webview/dist"]
pub struct RenderedDistAssets;

#[derive(RustEmbed)]
#[folder = "../apps/rendered-webview/src"]
pub struct RenderedSrcAssets;

#[derive(RustEmbed)]
#[folder = "../apps/rendered-webview/img"]
pub struct RenderedImgAssets;

#[derive(RustEmbed)]
#[folder = "../apps/background-webview/dist"]
pub struct BackgroundDistAssets;
```

Compiled into the binary. `RenderedDistAssets::get("rendered-auth.js")` returns file bytes at runtime with no filesystem access.

## Request Routing

File: `services/protocol_service.rs:46-104`

```
handle_request(url, state)
    |
    +-- url contains ".."? --> 403 (path traversal)
    +-- parse URL
    +-- host ends with ".passbolt.local"? --> no: 403
    +-- extract UUID from host, match to Rendered or Background
    +-- handle_webview_request(path, target, uuid, trusted_domain)
```

File: `services/protocol_service.rs:107-163`

```
handle_webview_request(path, target, uuid, trusted_domain)
    |
    +-- strip "apps/{webview}/" prefix from path
    |
    +-- path = "index-{mode}.html"?
    |   -> generate_index_html() with per-mode CSP
    |   -> Cache-Control: no-cache, no-store, must-revalidate
    |
    +-- path starts with "dist/"?
    |   -> serve from RenderedDistAssets or BackgroundDistAssets
    |   -> Cache-Control: public, max-age=31536000, immutable
    |
    +-- path starts with "src/"? (Rendered only)
    |   -> serve from RenderedSrcAssets
    |
    +-- path starts with "img/"? (Rendered only)
    |   -> serve from RenderedImgAssets
    |
    +-- fallback: try dist/
```

## Dynamic Index HTML

File: `services/local_folder_service.rs`

```rust
pub enum WebviewTarget { Rendered, Background }

pub struct IndexContext<'a> {
    pub target: &'a WebviewTarget,
    pub mode: &'a AppMode,
    pub uuid: &'a str,
    pub trusted_domain: Option<&'a str>,
    pub isolation_schema: Option<&'a str>,
}

pub fn generate_index_html(ctx: &IndexContext) -> String { ... }
pub fn generate_csp(ctx: &IndexContext) -> String { ... }
```

Index HTML is built in-memory with:
- `<meta http-equiv="Content-Security-Policy" content="...">` per (target x mode)
- `<script src="dist/{bundle}.js">` matching the mode

CSP varies by target and mode. See [[09 - Security Layers]] for the full CSP matrix.

## Locale Fallback

Locale files in the styleguide use a `passbolt-` prefix. The protocol handler tries the prefixed variant on 404:

```rust
// asset_path = "locales/en-UK/common.json"  -> not found
// try:        "locales/passbolt-en-UK/common.json"  -> found
```

## MIME Types

File: `services/protocol_service.rs:265-281`

Maps by file extension: `.js` -> `application/javascript`, `.css` -> `text/css`, `.woff2` -> `font/woff2`, `.svg` -> `image/svg+xml`, etc.
