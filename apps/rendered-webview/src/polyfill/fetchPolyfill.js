/**
 * Fetch polyfill for Rendered webview — routes passbolt:// requests through IPC.
 *
 * WKWebView blocks fetch() from custom protocol schemes. This polyfill
 * intercepts passbolt:// fetch calls and loads the embedded asset via IPC.
 * Binary assets are base64-encoded by Rust and decoded here.
 */

const originalFetch = window.fetch.bind(window);

// Lazy reference to the IPCHandler — set by the entry point after creation
let _port = null;

export function setFetchPolyfillPort(port) {
  _port = port;
}

window.fetch = async function(input, init = {}) {
  const url = typeof input === 'string'
    ? input
    : (input instanceof Request ? input.url : String(input));

  if (!url.startsWith('passbolt://') || !_port) {
    return originalFetch(input, init);
  }

  // Extract the asset path: strip origin + /apps/rendered-webview/dist/
  const pathMatch = url.match(/\.passbolt\.local\/apps\/rendered-webview\/dist\/(.+)$/);
  if (!pathMatch) {
    return originalFetch(input, init);
  }

  const assetPath = pathMatch[1];
  const result = await _port.request('passbolt.rendered.asset-fetch', assetPath);

  let body = result.body;
  if (result.encoding === 'base64') {
    const binary = atob(body);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    body = bytes;
  }

  return new Response(body, {
    status: result.status,
    headers: new Headers(result.headers || {}),
  });
};
