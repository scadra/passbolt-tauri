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
 * @since         2.8.0
 */
import FindTagsController from "passbolt-browser-extension/src/all/background_page/controller/tag/findTagsController";
import UpdateTagController from "passbolt-browser-extension/src/all/background_page/controller/tag/updateTagController";
import DeleteTagController from "passbolt-browser-extension/src/all/background_page/controller/tag/deleteTagController";
import UpdateResourceTagsController from "passbolt-browser-extension/src/all/background_page/controller/tag/updateResourceTagsController";
import AddTagsToResourcesController from "passbolt-browser-extension/src/all/background_page/controller/tag/addTagsToResourcesController";

const listen = function(worker, apiClientOptions) {
  worker.port.on("passbolt.tags.find-all", async (requestId) => {
    const findTagsController = new FindTagsController(worker, requestId, apiClientOptions);
    await findTagsController._exec();
  });

  worker.port.on("passbolt.tags.update", async (requestId, tagDto) => {
    const updateTagController = new UpdateTagController(worker, requestId, apiClientOptions);
    await updateTagController._exec(tagDto);
  });

  worker.port.on("passbolt.tags.delete", async (requestId, tagId) => {
    const deleteTagController = new DeleteTagController(worker, requestId, apiClientOptions);
    await deleteTagController._exec(tagId);
  });

  worker.port.on("passbolt.tags.update-resource-tags", async (requestId, resourceId, tagsDto) => {
    const updateResourceTagsController = new UpdateResourceTagsController(worker, requestId, apiClientOptions);
    await updateResourceTagsController._exec(resourceId, tagsDto);
  });

  worker.port.on("passbolt.tags.add-resources-tag", async (requestId, resourcesTagDto) => {
    const addTagsToResourcesController = new AddTagsToResourcesController(worker, requestId, apiClientOptions);
    await addTagsToResourcesController._exec(resourcesTagDto.resources, [resourcesTagDto.tag]);
  });
};

export const TagEvents = {listen};
