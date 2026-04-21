# Plan : Remplacer `api://` par un proxy HTTP via le bus d'events IPC

## Context

Le `fetch()` ne fonctionne pas dans la background webview parce que WKWebView (macOS) bloque les requetes HTTPS depuis un scheme custom (`passbolt://`). Le protocole `api://` avait ete cree pour contourner ce probleme, mais il n'est jamais utilise cote JS (le fetch polyfill ne fait que logger). De plus, l'extension browser exige que le trusted domain soit `https://`, donc recrire les URLs en `api://` n'est pas viable.

**Solution** : router les `fetch()` HTTPS a travers le systeme d'events IPC existant (deja autorise par l'isolation Tauri), ou Rust valide le domaine et fait la requete via reqwest. Cela supprime `api://` entierement.

## Flux cible

```
Background JS: fetch('https://passbolt.example.com/api/users.json')
  |  (fetchPolyfill intercepte)
  v
IPCHandler.request('passbolt.http.fetch', {url, method, headers, body})
  |  emit('background-ipc-message', {topic, requestId, message})
  v
Rust: BackgroundTopicService::handle() → match topic → handle_http_fetch()
  |  tauri::async_runtime::spawn(async { reqwest + cookie harvest })
  v
Rust: IpcMessage::success(requestId, {status, headers, body})
  |  emit_to('background-webview', 'background-ipc-forward', response)
  v
Background JS: IPCHandler._onMessage → once(requestId) callback
  |  resolve(new Response(body, {status, headers}))
  v
Extension recoit un Response standard, transparent
```

## Etapes d'implementation

### 1. Rust — Nouveau topic et handler HTTP fetch

**`src-tauri/src/config.rs`** — Ajouter constante :
```rust
pub const BACKGROUND_HTTP_FETCH: &str = "passbolt.http.fetch";
```

**`src-tauri/src/services/topics/background_handlers.rs`** — Nouveau handler :
```rust
pub fn handle_http_fetch(app: &AppHandle, state: &AppState, ipc: &IpcMessage) {
    // 1. Extraire requestId (obligatoire)
    // 2. Extraire request_data de ipc.message (array-unwrap comme handle_get_cookie)
    //    → { url: String, method: String, headers: Object, body: Option<String> }
    // 3. Cloner trusted_domain, cookies, app handle
    // 4. tauri::async_runtime::spawn(async move { ... })
    //    a. Valider trusted domain (reutiliser la logique de http_proxy_service)
    //    b. reqwest call
    //    c. Cookie harvesting
    //    d. Construire response JSON: { status: u16, headers: Object, body: String }
    //    e. IpcMessage::success(request_id, response) → forward_to_background
    //    f. En cas d'erreur: IpcMessage::error(request_id, error_msg)
}
```

**`src-tauri/src/services/background_topic_service.rs`** — Router le topic :
```rust
config::BACKGROUND_HTTP_FETCH => bg::handle_http_fetch(app, state, &ipc),
```

### 2. Rust — Refactorer http_proxy_service.rs

Renommer en `http_fetch_service.rs` (ou garder le nom et adapter).

Changer l'interface :
- **Avant** : `handle_api_protocol(url, method, headers, body, trusted_domain, cookies) -> Response<Vec<u8>>`
- **Apres** : `handle_fetch(request_data: &serde_json::Value, trusted_domain: Option<&str>, cookies: &CookieState) -> serde_json::Value`

La logique interne reste identique :
- Trusted domain check (lignes 34-45)
- PWNEDPASSWORDS whitelist (ligne 38)
- OPTIONS preflight (lignes 54-66)
- reqwest client (lignes 68-86)
- Cookie harvesting (lignes 102-114)
- Response building (lignes 117-140)

Retourne un JSON au lieu de `Response<Vec<u8>>` :
```json
{
  "status": 200,
  "headers": {"content-type": "application/json", "x-csrf-token": "..."},
  "body": "<response body as string>"
}
```

### 3. Rust — Supprimer `api://`

**`src-tauri/src/main.rs`** :
- Supprimer lignes 34-55 (`register_asynchronous_uri_scheme_protocol` pour `API_PROTOCOL_SCHEME`)
- Supprimer `http_proxy_service` de l'import ligne 5

**`src-tauri/src/config.rs`** :
- Supprimer `API_PROTOCOL_SCHEME` et son commentaire (lignes 4-6)

**`src-tauri/src/services/mod.rs`** :
- Remplacer `pub mod http_proxy_service` par `pub mod http_fetch_service` (si renommage)

### 4. Rust — Nettoyer la CSP

**`src-tauri/src/services/local_folder_service.rs`** :

- Supprimer `format_api_domain()` (lignes 226-234)
- Dans `generate_background_csp()`, retirer `{api_domain}` du `connect-src` pour Auth et Workspace (lignes 162, 167-169)
- Optionnel : retirer aussi `{domain}` du connect-src (puisque le fetch passe par IPC, pas par HTTP direct). Cela rend la CSP plus restrictive = plus securisee.

CSP Background apres nettoyage :
```
connect-src ipc://localhost/... {base}dist/locales/
```
(identique pour Import, Auth et Workspace — plus de distinction par mode pour connect-src)

### 5. JS — Nouveau fetch polyfill

**`apps/background-webview/src/polyfill/fetchPolyfill.js`** (recreer) :

```javascript
import IPCHandler from '../shared/IPCHandler';

const originalFetch = window.fetch.bind(window);

window.fetch = async function(input, init = {}) {
  const url = typeof input === 'string'
    ? input
    : (input instanceof Request ? input.url : String(input));
  const method = init.method
    || (input instanceof Request ? input.method : 'GET');

  // Seules les requetes HTTPS passent par le proxy Rust
  if (!url.startsWith('https://')) {
    return originalFetch(input, init);
  }

  // Normaliser les headers en objet plat
  const headers = {};
  if (init.headers) {
    if (init.headers instanceof Headers) {
      init.headers.forEach((v, k) => { headers[k] = v; });
    } else if (Array.isArray(init.headers)) {
      init.headers.forEach(([k, v]) => { headers[k] = v; });
    } else {
      Object.assign(headers, init.headers);
    }
  }

  // Router via IPC → Rust → reqwest
  const ipc = IPCHandler.getInstance();
  const result = await ipc.request('passbolt.http.fetch', {
    url,
    method,
    headers,
    body: init.body || null,
  });

  // Reconstruire un objet Response standard
  return new Response(result.body, {
    status: result.status,
    statusText: result.statusText || '',
    headers: new Headers(result.headers || {}),
  });
};
```

**Note timing** : le polyfill override `window.fetch` au load, mais `IPCHandler.getInstance()` n'est appele qu'au moment d'un vrai fetch (apres initialisation). Le singleton IPCHandler est cree par Main.constructor, bien avant le premier fetch de l'extension.

### 6. JS — Importer le polyfill dans les entry points

**`apps/background-webview/index-auth.js`**, **`index-workspace.js`**, **`index-import.js`** :

Ajouter apres les autres polyfills :
```javascript
import './src/polyfill/fetchPolyfill';
```

### 7. Tests

**Rust — mettre a jour** :
- `tests/services/local_folder_service.rs` : retirer les assertions `api://` dans les CSP, verifier que connect-src ne contient plus le trusted domain
- `tests/services/protocol_service.rs` : aucun changement (ne concerne pas api://)
- Nouveau test : `handle_http_fetch` handler (trusted domain accepted, untrusted rejected, cookie harvesting, response format)

**JS — nouveau** :
- `apps/background-webview/src/polyfill/fetchPolyfill.test.js` : URL interception (https:// → IPC, passbolt:// → passthrough), Response construction, header normalization

## Fichiers modifies

| Fichier | Action |
|---------|--------|
| `src-tauri/src/config.rs` | Ajouter `BACKGROUND_HTTP_FETCH`, supprimer `API_PROTOCOL_SCHEME` |
| `src-tauri/src/main.rs` | Supprimer handler `api://` (lignes 34-55), supprimer import |
| `src-tauri/src/services/mod.rs` | Renommer module |
| `src-tauri/src/services/http_proxy_service.rs` | Refactorer → `http_fetch_service.rs` |
| `src-tauri/src/services/background_topic_service.rs` | Ajouter route `BACKGROUND_HTTP_FETCH` |
| `src-tauri/src/services/topics/background_handlers.rs` | Ajouter `handle_http_fetch()` |
| `src-tauri/src/services/local_folder_service.rs` | Supprimer `format_api_domain()`, nettoyer CSP |
| `apps/background-webview/src/polyfill/fetchPolyfill.js` | Recreer avec proxy IPC |
| `apps/background-webview/index-auth.js` | Ajouter import fetchPolyfill |
| `apps/background-webview/index-workspace.js` | Ajouter import fetchPolyfill |
| `apps/background-webview/index-import.js` | Ajouter import fetchPolyfill |
| Tests Rust CSP | Mettre a jour assertions |

## Verification

1. `cargo clippy --manifest-path=src-tauri/Cargo.toml` — pas de warnings
2. `cargo test --manifest-path=src-tauri/Cargo.toml` — tous les tests passent
3. `cd apps/background-webview && npm test` — tests JS passent
4. `npm run build:webviews` — build OK
5. `npm run tauri:dev:trace` — lancer l'app, observer dans les logs :
   - `[BackgroundTopic] Processing: passbolt.http.fetch` quand l'extension fait un appel API
   - `[HTTP] >>> GET https://...` dans les logs reqwest
   - L'authentification complete fonctionne (login flow)
