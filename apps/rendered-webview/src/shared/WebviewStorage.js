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
 * @since         0.0.1
 */

import { renderedStore } from "./IPCHandler";

/**
 * Storage abstraction for the Rendered webview.
 *
 * Reads from renderedStore (in-memory Map) fed by IPCHandler._handleEarlyStorageSync.
 * No size limit — handles resources (~15MB) without hitting sessionStorage/localStorage quotas.
 */
class WebviewStorage {
  constructor() {
    this._listeners = [];
    this._storageEventListener = null;
  }

  get storage() {
    return {
      local: {
        get: this.getStorage.bind(this),
        set: this.setStorage.bind(this),
        remove: this.removeStorage.bind(this),
        clear: this.clearStorage.bind(this),
      },
      onChanged: {
        addListener: callback => {
          if (!this._listeners.includes(callback)) {
            this._listeners.push(callback);

            if (!this._storageEventListener) {
              this._storageEventListener = event => {
                const changes = {};
                const key = event.key;

                if (key !== null) {
                  try {
                    changes[key] = {
                      newValue: JSON.parse(event.newValue),
                    };
                  } catch {
                    changes[key] = {newValue: event.newValue};
                  }

                  this._listeners.forEach(listener => listener(changes));
                }
              };

              window.addEventListener("storage", this._storageEventListener);
            }
          }
        },

        removeListener: callback => {
          const index = this._listeners.indexOf(callback);
          if (index !== -1) {
            this._listeners.splice(index, 1);
          }

          if (this._listeners.length === 0 && this._storageEventListener) {
            window.removeEventListener(
              "storage",
              this._storageEventListener
            );
            this._storageEventListener = null;
          }
        },
      },
    };
  }

  async getStorage(key) {
    const actualKey = Array.isArray(key) ? key[0] : key;
    // renderedStore first, sessionStorage fallback for _passbolt_data after navigation
    const value = renderedStore.get(actualKey) ?? sessionStorage.getItem(actualKey) ?? null;
    try {
      return {[actualKey]: value !== null ? JSON.parse(value) : null};
    } catch {
      return {[actualKey]: value};
    }
  }

  setStorage(key, value) {
    renderedStore.set(key, JSON.stringify(value));
    this._triggerStorageEvent(key, JSON.stringify(value));
  }

  removeStorage(key) {
    renderedStore.delete(key);
    this._triggerStorageEvent(key, null);
  }

  clearStorage() {
    renderedStore.clear();
    this._triggerStorageEvent(null, null);
  }

  _triggerStorageEvent(key, newValue) {
    if (this._storageEventListener) {
      const event = new Event("storage");
      event.key = key;
      event.newValue = newValue;
      this._storageEventListener(event);
    }
  }
}


export default WebviewStorage;
