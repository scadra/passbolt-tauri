/**
 * External link polyfill — opens external URLs in the system browser.
 *
 * Intercepts both window.open() calls and <a> tag clicks with external URLs,
 * routing them through @tauri-apps/plugin-opener. The URL whitelist is enforced
 * by Tauri capabilities (capabilities/default.json → opener:allow-open-url).
 *
 * The isolation iframe allows plugin:opener|open_url (src/isolation/index.js).
 */
import { openUrl } from '@tauri-apps/plugin-opener';

/**
 * Check if a URL points to an external origin (http/https, different host).
 */
function isExternalUrl(href) {
  if (!href) return false;
  try {
    const urlObj = new URL(href, window.location.href);
    return (urlObj.protocol === 'http:' || urlObj.protocol === 'https:')
      && urlObj.host !== window.location.host;
  } catch (error) {
    return /^https?:\/\//.test(href);
  }
}

/**
 * Open an external URL in the system browser via the Tauri opener plugin.
 * Capabilities whitelist controls which URLs are allowed.
 */
function openExternal(url) {
  if (process.env.NODE_ENV !== 'production') {
    console.log('[externalLinkPolyfill] Opening external URL:', url);
  }
  openUrl(url).catch(error => {
    if (process.env.NODE_ENV !== 'production') {
      console.warn('[externalLinkPolyfill] opener failed:', error);
    }
  });
}

// --- Override window.open() for external URLs ---
const originalWindowOpen = window.open.bind(window);

window.open = function(url, target, features) {
  if (url && isExternalUrl(url)) {
    openExternal(url);
    return { closed: true, close: () => {}, focus: () => {}, blur: () => {}, postMessage: () => {} };
  }
  return originalWindowOpen(url, target, features);
};

// --- Intercept <a> clicks with external URLs ---
function findAnchorAncestor(element) {
  let current = element;
  while (current && current !== document) {
    if (current.tagName === 'A') return current;
    current = current.parentElement;
  }
  return null;
}

document.addEventListener('click', (event) => {
  const anchor = findAnchorAncestor(event.target);
  if (!anchor) return;

  const href = anchor.getAttribute('href');
  if (!isExternalUrl(href)) return;

  event.preventDefault();
  event.stopPropagation();
  openExternal(href);
}, true); // capture phase — intercept before React handlers

console.log('[externalLinkPolyfill] Installed');
