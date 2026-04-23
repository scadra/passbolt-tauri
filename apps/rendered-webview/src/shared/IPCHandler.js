/**
 * Passbolt ~ Open source password manager for teams
 * Copyright (c) 2023 Passbolt SA (https://www.passbolt.com)
 *
 * Licensed under GNU Affero General Public License version 3 of the or any later version.
 * For full copyright and license information, please see the LICENSE.txt
 * Redistributions of files must retain the above copyright notice.
 *
 * @copyright     Copyright (c) 2023 Passbolt SA (https://www.passbolt.com)
 * @license       https://opensource.org/licenses/AGPL-3.0 AGPL License
 * @link          https://www.passbolt.com Passbolt(tm)
 * @since         0.0.1
 */

import { listen, emit as tauriEmit } from '@tauri-apps/api/event';
import { v4 as uuidv4 } from "uuid";

/**
 * In-memory store for the Rendered webview — fed by IPC sync from Background.
 * Exported so WebviewStorage can read from it.
 */
export const renderedStore = new Map();

class IPCHandler {
  constructor() {
    this._listeners = {};
    this._eventListeners = new Map();
    this._pendingRequestIds = new Set();
    this._activeRequests = 0;
    this._maxConcurrentRequests = 1;
    this._requestQueue = [];
    this.initListener();
  }

  async initListener() {
    const unlisten = await listen('rendered-ipc-forward', (event) => {
      this._onMessage(event);
    });
    this._globalUnlisten = unlisten;

    // Clean up the Tauri event listener on page unload (mode transitions).
    // Without this, the old listener leaks when the page reloads for
    // import→auth→workspace transitions, causing duplicate handlers.
    window.addEventListener('beforeunload', () => {
      this.destroy();
    });
  }

  /**
   * Release all Tauri event listeners and clear internal state.
   * Called automatically on page unload to prevent stale listener accumulation.
   */
  destroy() {
    if (this._globalUnlisten) {
      this._globalUnlisten();
      this._globalUnlisten = null;
    }
    this._listeners = {};
    this._eventListeners.clear();
    this._pendingRequestIds.clear();
    this._requestQueue = [];
  }

  _onMessage(event) {
    const eventData = event.payload;
    const eventName = eventData.topic ? eventData.topic : eventData.requestId;
    console.log('[Rendered IPC] Received event:', eventName, 'message:', eventData.message, 'hasListeners:', Array.isArray(this._listeners[eventName]));

    // Write localStorage updates to sessionStorage immediately, BEFORE checking
    // for listeners. This ensures the store is populated even if the
    // RenderedWebview component hasn't mounted yet (timing fix for _passbolt_data).
    this._handleEarlyStorageSync(eventData);

    if (Array.isArray(this._listeners[eventName])) {
      const listeners = this._listeners[eventName];
      for (let i = 0; i < listeners.length; i++) {
        const listener = listeners[i];
        if (eventData.requestId) {
          let args;
          if (eventData.status) {
            args = [eventData.status, eventData.message];
          } else if (Array.isArray(eventData.message)) {
            args = [eventData.requestId, ...eventData.message];
          } else if (eventData.withMultipleParam) {
            try {
              args = [eventData.requestId].concat(JSON.parse(eventData.message));
            } catch (error) {
              console.error(error);
              throw error;
            }
          } else {
            args = [eventData.requestId, eventData.message];
          }
          listener.callback.apply(this, args);
        } else if (eventData.status) {
          listener.callback.apply(this, [eventData.status, eventData.message]);
        } else {
          const message = Array.isArray(eventData.message) ? eventData.message : [eventData.message];
          listener.callback.apply(this, message);
        }
        if (listener.once) {
          this._listeners[eventName].splice(i, 1);
          if (this._listeners[eventName].length === 0) {
            delete this._listeners[eventName];
          }
          i--;
        }
      }
    }
  }

  /**
   * Write storage sync messages to the in-memory store immediately on arrival.
   * This runs BEFORE the listener dispatch, so the store is always up to date
   * even if RenderedWebview hasn't mounted yet (fixes _passbolt_data timing).
   *
   * Uses a module-level Map (renderedStore) — no size limit.
   */
  _handleEarlyStorageSync(eventData) {
    const topic = eventData.topic;
    if (topic === 'passbolt.rendered.localstorage-update' && eventData.message) {
      const {key, value} = eventData.message;
      if (key) {
        renderedStore.set(key, value);
        // _passbolt_data must survive page navigation (auth → workspace, MFA → workspace).
        // sessionStorage persists within the same webview session.
        if (key === '_passbolt_data') {
          try { sessionStorage.setItem(key, value); } catch {}
        }
        // Notify WebviewStorage.onChanged listeners so React contexts re-render.
        const storageEvent = new Event("storage");
        storageEvent.key = key;
        storageEvent.newValue = value;
        window.dispatchEvent(storageEvent);
      }
    } else if (topic === 'passbolt.rendered.localstorage-delete' && eventData.message) {
      const key = typeof eventData.message === 'object' ? eventData.message.key : eventData.message;
      renderedStore.delete(key);
      const storageEvent = new Event("storage");
      storageEvent.key = key;
      storageEvent.newValue = null;
      window.dispatchEvent(storageEvent);
    } else if (topic === 'passbolt.rendered.localstorage-clear') {
      renderedStore.clear();
    }
  }

  _addListener(name, callback, once) {
    if (!Array.isArray(this._listeners[name])) {
      this._listeners[name] = [];
    }
    this._listeners[name].push({
      name: name,
      callback: callback,
      once: once
    });
  }

  on(name, callback) {
    this._addListener(name, callback, false);
  }

  once(name, callback) {
    this._addListener(name, callback, true);
  }

  async emit(...requestArgs) {
    let ipc;

    if (typeof requestArgs[0] === 'string') {
      let status, message;

      if (requestArgs[1] === 'SUCCESS' || requestArgs[1] === 'ERROR') {
        status = requestArgs[1];
        message = requestArgs.length > 2 ? requestArgs[2] : null;
      } else {
        status = null;
        message = requestArgs[1];
      }
      ipc = {
        topic: requestArgs[0],
        status: status,
        message: message
      };
    } else {
      ipc = requestArgs[0];
    }

    await tauriEmit('rendered-ipc-message', ipc);
  }

  _releaseRequestSlot(requestId) {
    if (!this._pendingRequestIds.delete(requestId)) {
      return;
    }
    this._activeRequests--;
    if (this._requestQueue.length > 0) {
      const next = this._requestQueue.shift();
      next();
    }
  }

  async request(message, ...args) {
    if (this._activeRequests >= this._maxConcurrentRequests) {
      await new Promise(resolve => this._requestQueue.push(resolve));
    }
    this._activeRequests++;

    const requestId = uuidv4();
    this._pendingRequestIds.add(requestId);

    // Safety: release slot after 1s if no response (prevents deadlock)
    setTimeout(() => this._releaseRequestSlot(requestId), 1000);

    const requestArgs = [{topic: message, requestId: requestId, message: args}];

    return new Promise((resolve, reject) => {
      this.once(requestId, (status, ...callbackArgs) => {
        this._releaseRequestSlot(requestId);
        if (status === 'SUCCESS') {
          resolve.apply(null, callbackArgs);
        } else if (status === 'ERROR') {
          console.error('[Rendered IPC] request() ERROR topic:', message, 'error:', callbackArgs);
          reject.apply(null, callbackArgs);
        }
      });

      this.emit.apply(this, requestArgs);
    });
  }

  destroy() {
    if (this._globalUnlisten) {
      this._globalUnlisten();
    }
    this._eventListeners.forEach(unlisten => unlisten());
    this._eventListeners.clear();
    this._listeners = {};
  }
}

export default IPCHandler;
