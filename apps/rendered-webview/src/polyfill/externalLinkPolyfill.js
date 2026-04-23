/**
 * External link polyfill — delegates external URLs to Tauri's on_new_window handler.
 *
 * We cannot call @tauri-apps/plugin-opener from JS because the isolation iframe
 * only allows emit/listen/unlisten — opener commands fail with DataCloneError.
 * Instead, we call originalWindowOpen() so Tauri's on_new_window Rust handler fires.
 * The Rust handler checks the whitelist and opens allowed URLs in the system browser.
 */

const originalWindowOpen = window.open.bind(window);

window.open = function(url, target, features) {
  if (!url) {
    return originalWindowOpen(url, target, features);
  }

  let isExternal = false;
  try {
    const urlObj = new URL(url, window.location.href);
    isExternal = (urlObj.protocol === 'http:' || urlObj.protocol === 'https:')
      && urlObj.host !== window.location.host;
  } catch (error) {
    isExternal = /^https?:\/\//.test(url);
  }

  if (isExternal) {
    if (process.env.NODE_ENV !== 'production') {
      console.log('[externalLinkPolyfill] External URL → delegating to on_new_window:', url);
    }
    // Call original window.open so Tauri's on_new_window handler fires.
    // Rust will check the whitelist and open allowed URLs in the system browser.
    // on_new_window always returns Deny, so no new webview window is created.
    originalWindowOpen(url, target, features);
    // Return mock window to prevent caller errors (the real window is denied).
    return {
      closed: true,
      close: () => {},
      focus: () => {},
      blur: () => {},
      postMessage: () => {}
    };
  }

  return originalWindowOpen(url, target, features);
};

console.log('[externalLinkPolyfill] Installed');
