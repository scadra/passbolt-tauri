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

class IPCHandler {
  constructor() {
    this._listeners = {};
    this._eventListeners = new Map();
    this.initListener();
  }

  async initListener() {
    const unlisten = await listen('rendered-ipc-forward', (event) => {
      this._onMessage(event);
    });
    this._globalUnlisten = unlisten;
  }

  _onMessage(event) {
    const eventData = event.payload;
    const eventName = eventData.topic ? eventData.topic : eventData.requestId;
    console.log('[Rendered IPC] Received event:', eventName, 'status:', eventData.status, 'hasListeners:', Array.isArray(this._listeners[eventName]));

    if (Array.isArray(this._listeners[eventName])) {
      const listeners = this._listeners[eventName];
      for (let i = 0; i < listeners.length; i++) {
        const listener = listeners[i];
        if (eventData.requestId) {
          let args;
          if (Array.isArray(eventData.message)) {
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

  async request(message, ...args) {
    const requestId = uuidv4();
    console.log('[Rendered IPC] request() topic:', message, 'requestId:', requestId);
    const requestArgs = [{topic: message, requestId: requestId, message: args}];

    return new Promise((resolve, reject) => {
      this.once(requestId, (status, ...callbackArgs) => {
        if (status === 'SUCCESS') {
          resolve.apply(null, callbackArgs);
        } else if (status === 'ERROR') {
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