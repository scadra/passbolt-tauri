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
 * @since         0.3.0
 */

import ExportResourcesFileController from "../controllers/exportResourcesFilesController";
import FindExportPoliciesSettingsController from "passbolt-browser-extension/src/all/background_page/controller/exportPolicies/findExportPoliciesSettingsController";

const listen = function(worker, apiClientOptions, account) {
  worker.port.on('passbolt.export-resources.export-to-file', async(requestId, exportResourcesFileDto) => {
    const exportController = new ExportResourcesFileController(worker, apiClientOptions, requestId, account);
    await exportController.exec(exportResourcesFileDto);
  });

  worker.port.on("passbolt.export-policies.get", async requestId => {
    const controller = new FindExportPoliciesSettingsController(worker, requestId, apiClientOptions);
    await controller._exec();
  });
};

export const ExportResourcesEvents = {listen};
