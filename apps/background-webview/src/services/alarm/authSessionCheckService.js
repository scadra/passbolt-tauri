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
 * @since         1.1.0
 */

import { emit } from '@tauri-apps/api/event';
import CheckAuthStatusService from "passbolt-browser-extension/src/all/background_page/service/auth/checkAuthStatusService";

const AUTH_SESSION_CHECK_ALARM = "AuthSessionCheck";

class AuthSessionCheckService {
  static async handleAuthStatusCheckAlarm(alarm) {
    if (alarm.name !== AuthSessionCheckService.ALARM_NAME) {
      return;
    }

    const checkAuthService = new CheckAuthStatusService();
    const authStatus = await checkAuthService.checkAuthStatus(true);
    if (!authStatus.isAuthenticated) {
      await emit("passbolt.auth.after-logout", {});
    }
  }

  static get ALARM_NAME() {
    return AUTH_SESSION_CHECK_ALARM;
  }
}

export default AuthSessionCheckService;
