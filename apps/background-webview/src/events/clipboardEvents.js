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
 * @since         2.3.0
 */

import CopyToClipboardController from "passbolt-browser-extension/src/all/background_page/controller/clipboard/copyToClipboardController";

const listen = function(worker) {
  worker.port.on('passbolt.clipboard.copy', async(requestId, text) => {
    console.log('[CLIPBOARD] clipboardEvents received passbolt.clipboard.copy, requestId:', requestId, 'text length:', text?.length);
    const clipboardController = new CopyToClipboardController(worker, requestId);
    await clipboardController._exec(text);
    console.log('[CLIPBOARD] clipboardEvents _exec done');
  });

  worker.port.on('passbolt.clipboard.copy-temporarily', async(requestId, text) => {
    const clipboardController = new CopyToClipboardController(worker, requestId);
    await clipboardController._exec(text);
  });
};

export const ClipboardEvents = {listen};
