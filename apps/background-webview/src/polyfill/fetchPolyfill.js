/**
 * Fetch polyfill — routes HTTPS requests through Rust via IPC.
 *
 * On macOS, WKWebView blocks fetch() from custom protocol schemes (passbolt://).
 * This polyfill intercepts all HTTPS fetch calls and proxies them through the
 * IPC event system to Rust, which makes the actual request via reqwest.
 *
 * Non-HTTPS URLs (passbolt://, data:, blob:, etc.) pass through to the original fetch.
 * On Windows with useHttpsScheme, IPC URLs (ipc.localhost) also pass through.
 */
import IPCHandler from '../shared/IPCHandler';

const originalFetch = window.fetch.bind(window);

console.log('[fetchPolyfill] Installed — window.fetch replaced');

window.fetch = async function(input, init = {}) {
  const url = typeof input === 'string'
    ? input
    : (input instanceof Request ? input.url : String(input));
  const method = init.method
    || (input instanceof Request ? input.method : 'GET');

  // Only proxy HTTPS requests — everything else passes through.
  // Also exclude ipc.localhost which is Tauri's internal IPC on Windows
  // (with useHttpsScheme, IPC uses https://ipc.localhost/).
  if (!url.startsWith('https://') || url.includes('ipc.localhost')) {
    return originalFetch(input, init);
  }

  // Normalize headers to a plain object
  const headers = {};
  const sourceHeaders = init.headers || (input instanceof Request ? input.headers : null);
  if (sourceHeaders) {
    if (sourceHeaders instanceof Headers) {
      sourceHeaders.forEach((value, key) => { headers[key] = value; });
    } else if (Array.isArray(sourceHeaders)) {
      sourceHeaders.forEach(([key, value]) => { headers[key] = value; });
    } else {
      Object.assign(headers, sourceHeaders);
    }
  }

  // Extract body as string — must be serializable for structured clone (IPC)
  let body = init.body ?? null;
  if (!body && input instanceof Request) {
    try { body = await input.clone().text(); } catch { body = null; }
  }
  if (body instanceof ArrayBuffer || body instanceof Uint8Array) {
    body = new TextDecoder().decode(body);
  } else if (body instanceof FormData) {
    body = new URLSearchParams(body).toString();
    if (!headers['content-type']) {
      headers['content-type'] = 'application/x-www-form-urlencoded';
    }
  } else if (body instanceof URLSearchParams) {
    body = body.toString();
  } else if (body instanceof Blob) {
    body = await body.text();
  } else if (body !== null && typeof body !== 'string') {
    try { body = await new Response(body).text(); } catch { body = null; }
  }

  console.log('[fetchPolyfill] >>>', method, url);

  // Route through IPC → Rust → reqwest
  const ipc = IPCHandler.getInstance();
  const result = await ipc.request('passbolt.http.fetch', {
    url,
    method,
    headers,
    body,
  });

  console.log('[fetchPolyfill] <<<', method, url, 'status:', result.status);

  // Reconstruct a standard Response object
  return new Response(result.body, {
    status: result.status,
    headers: new Headers(result.headers || {}),
  });
};
