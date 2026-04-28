import './assets_registry.js';
import { initializeGlobals } from './compat.js';
import { AppController } from './app_controller.js';
import { ModalManager } from './utils/ModalManager.js';
import { DOM_IDS } from './constants_dom.js';

export async function initialize() {
    initializeGlobals({ restartPolling: AppController.restartPolling });
    await AppController.initialize();
}
