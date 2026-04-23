/**
 * Passbolt ~ Open source password manager for teams
 * Copyright (c) Passbolt SA (https://www.passbolt.com)
 *
 * Licensed under GNU Affero General Public License version 3 of the or any later version.
 * For full copyright and license information, please see the LICENSE.txt
 * Redistributions of files must retain the above copyright notice.
 *
 * @copyright     Copyright (c) Passbolt SA (https://www.passbolt.com)
 * @license       https://opensource.org/licenses/AGPL-3.0 AGPL License
 * @link          https://www.passbolt.com Passbolt(tm)
 * @since         0.8.0
 */

/**
 * Browser security polyfill — equivalent of C# SetWebviewSettings() for
 * keyboard shortcuts and context menu.
 *
 * Two responsibilities:
 *
 * 1. Block browser keyboard shortcuts (AreBrowserAcceleratorKeysEnabled = false)
 *    - Reload (Cmd+R / Ctrl+R / F5)
 *    - View source (Cmd+U / Ctrl+U)
 *    - Find (Cmd+F / Ctrl+F / F3)
 *    - Print (Cmd+P / Ctrl+P)
 *    - Open file (Cmd+O / Ctrl+O)
 *    - Downloads (Cmd+J / Ctrl+J)
 *
 * 2. Block the native browser context menu (AreDefaultContextMenusEnabled = false)
 *    The styleguide's custom React context menus (ContextualMenuContext.show())
 *    still work because they call show() in their own onContextMenu handler
 *    BEFORE this capture-phase listener fires preventDefault().
 *
 * Does NOT block:
 * - Cmd+C / Cmd+V / Cmd+X — clipboard operations (needed by the app)
 * - Cmd+A — select all (useful in forms)
 * - Cmd+Z / Cmd+Shift+Z — undo/redo (useful in forms)
 * - Tab / Shift+Tab — focus navigation (accessibility)
 * - DevTools (Cmd+Shift+I) — controlled at native level via .devtools(false)
 *
 * Works cross-platform: uses metaKey (Cmd on macOS) and ctrlKey (Ctrl on Windows/Linux).
 */

// ── Keyboard shortcuts ─────────────────────────────────────────

const BLOCKED_SHORTCUTS = new Set([
  // Reload
  'meta+r', 'ctrl+r', 'meta+shift+r', 'ctrl+shift+r',
  // View source
  'meta+u', 'ctrl+u',
  // Find
  'meta+f', 'ctrl+f', 'meta+g', 'ctrl+g', 'meta+shift+g', 'ctrl+shift+g',
  // Print
  'meta+p', 'ctrl+p',
  // Open file
  'meta+o', 'ctrl+o',
  // Downloads
  'meta+j', 'ctrl+j',
]);

// F-keys blocked regardless of modifiers
const BLOCKED_FKEYS = new Set([
  'F3',      // Find
  'F5',      // Reload
  'F10',     // Windows context menu (Shift+F10)
]);

window.addEventListener('keydown', (event) => {
  // Block specific F-keys
  if (BLOCKED_FKEYS.has(event.key)) {
    event.preventDefault();
    event.stopImmediatePropagation();
    return;
  }

  // Build modifier+key combo string (order: meta → ctrl → shift → alt → key)
  const parts = [];
  if (event.metaKey) parts.push('meta');
  if (event.ctrlKey) parts.push('ctrl');
  if (event.shiftKey) parts.push('shift');
  if (event.altKey) parts.push('alt');
  parts.push(event.key.toLowerCase());
  const combo = parts.join('+');

  if (BLOCKED_SHORTCUTS.has(combo)) {
    event.preventDefault();
    event.stopImmediatePropagation();
  }
}, { capture: true });

// ── Context menu ───────────────────────────────────────────────
//
// Blocks the native browser context menu (WKWebView "Reload" / "Inspect Element").
// The styleguide's custom React menus work because their onContextMenu handlers
// call contextualMenuContext.show() synchronously — by the time this capture-phase
// listener fires, the React menu is already queued for render.

document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
  return false;
}, { capture: true });
