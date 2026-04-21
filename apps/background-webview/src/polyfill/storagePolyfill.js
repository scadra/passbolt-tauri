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

import {LOCALSTORAGE_CLEAR, LOCALSTORAGE_DELETE, LOCALSTORAGE_UPDATE} from "../enumerations/appEventEnumeration";
import { emit } from '@tauri-apps/api/event';

/**
 * Polyfill to match the webview storage with the bext storage.
 *
 * localStorage writes are batched: multiple rapid `set()` calls are
 * accumulated and flushed as individual IPC messages after a microtick.
 * This prevents saturating the macOS main thread with protocol handler
 * callbacks when the passbolt-browser-extension writes ~16 keys at init.
 */
class StoragePolyfill {
  constructor() {
    this._pendingUpdates = [];
    this._flushScheduled = false;
    this.initLocaleStorage();
  }

  /**
   * init the chrome storage with the localstorage webview
   */
  initLocaleStorage() {
    window.chrome.storage = {
      local: {
        get: this.getStorage,
        set: (storage, value = "") => {
          if (typeof storage === "object") {
            const keys = Object.keys(storage);
            const values = Object.values(storage);
            localStorage.setItem(keys[0], JSON.stringify(values[0]));
            this._queueUpdate(keys[0], JSON.stringify(values[0]));
          } else {
            localStorage.setItem(storage, value);
            this._queueUpdate(storage, value);
          }
        },
        remove: key => {
          localStorage.removeItem(key);
          this.onStorageDelete(key);
        },
        clear: () => {
          localStorage.clear();
          this.onStorageCleared();
        }
      }
    };
  }

  /**
   * Queue an update for batched delivery. The flush happens after a 50ms
   * debounce — all synchronous and near-synchronous writes are coalesced
   * into a single batch. This prevents saturating the macOS main thread.
   */
  _queueUpdate(key, value) {
    // Deduplicate: only keep the latest value per key
    const existing = this._pendingUpdates.findIndex(u => u.key === key);
    if (existing !== -1) {
      this._pendingUpdates[existing].value = value;
    } else {
      this._pendingUpdates.push({key, value});
    }

    // Reset the debounce timer on every write
    if (this._flushTimer) {
      clearTimeout(this._flushTimer);
    }
    this._flushTimer = setTimeout(() => this._flush(), 50);
  }

  /**
   * Flush all pending updates as individual IPC messages.
   */
  async _flush() {
    const updates = this._pendingUpdates;
    this._pendingUpdates = [];
    this._flushTimer = null;

    for (const {key, value} of updates) {
      await emit('background-ipc-message', {topic: LOCALSTORAGE_UPDATE, message: {key, value}});
    }
  }

  /**
   * Match the get storage from the localstorage of the webview
   * @param {string} key
   * @param {function} callback
   * @returns
   */
  getStorage(key, callback) {
    return new Promise((resolve, reject) => {
      try {
        let localKey = key;
        if (Array.isArray(key)) {
          localKey = key[0];
        }
        const value = localStorage.getItem(localKey);
        const response = value !== null ? value : undefined;
        if (callback) {
          callback({_passbolt_data: response ? JSON.parse(response) : undefined});
        }

        const result = {};
        result[localKey] = response ? JSON.parse(response) : undefined;
        resolve(result);
      } catch (error) {
        reject(error);
      }
    });
  }

  /**
   * Send event whenever storage has been cleared
   */
  async onStorageCleared() {
    await emit('background-ipc-message', {topic: LOCALSTORAGE_CLEAR});
  }

  /**
   * Send event whenever storage has a entry deleted
   * @param {string} key
   */
  async onStorageDelete(key) {
    await emit('background-ipc-message', {topic: LOCALSTORAGE_DELETE, message: key});
  }
}

/**
 * Init the polyfill only if runtime is missing
 */
if (!window.chrome.storage?.local) {
  new StoragePolyfill();
}
