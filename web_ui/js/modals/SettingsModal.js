import { State } from '../state.js';
import { Network } from '../network.js';
import * as i18n from '../i18n/index.js';
import { ModalManager } from '../utils/ModalManager.js';

const SETTINGS_MODAL_ID = 'settings-modal';
const LANGUAGE_BUTTON_ID = 'lang-btn';

function getTranslationMap() {
    return window.translations || null;
}

function getLanguageLabel() {
    return State.currentLang === 'jp' ? 'English' : 'Japanese';
}

export const SettingsModal = {
    openSettingsModal: () => {
        ModalManager.show(SETTINGS_MODAL_ID);
    },

    closeSettingsModal: () => {
        ModalManager.hide(SETTINGS_MODAL_ID);
    },

    updateBoardScale: (val) => {
        const scale = parseFloat(val);
        document.documentElement.style.setProperty('--zoom-override', scale);

        const zoomValue = document.getElementById('zoom-value');
        if (zoomValue) {
            zoomValue.textContent = scale.toFixed(2);
        }

        const slider = document.getElementById('zoom-slider');
        if (slider && slider.value !== String(val)) {
            slider.value = String(val);
        }

        localStorage.setItem('lovelive_board_scale', String(scale));
    },

    toggleLang: async () => {
        State.currentLang = State.currentLang === 'jp' ? 'en' : 'jp';
        localStorage.setItem('lovelive_lang', State.currentLang);

        await i18n.loadTranslations(State.currentLang);
        SettingsModal.updateLanguage();
    },

    toggleFriendlyAbilities: () => {
        State.showFriendlyAbilities = !State.showFriendlyAbilities;
        localStorage.setItem('lovelive_friendly_abilities', String(State.showFriendlyAbilities));
        SettingsModal.updateLanguage();
        window.render?.();
    },

    updateLanguage: () => {
        const translations = getTranslationMap();
        const langData = translations?.[State.currentLang];
        const ui = langData?.ui || {};

        document.querySelectorAll('[data-i18n]').forEach((element) => {
            const key = element.getAttribute('data-i18n');
            if (!key || !ui[key]) {
                return;
            }

            if (key === 'friendly_abilities' || key === 'live_watch') {
                const isEnabled = key === 'friendly_abilities'
                    ? State.showFriendlyAbilities
                    : State.isLiveWatchOn;
                const stateLabel = isEnabled ? (ui.on || 'ON') : (ui.off || 'OFF');
                element.textContent = `${ui[key]}: ${stateLabel}`;
                return;
            }

            element.textContent = ui[key];
        });

        const languageButton = document.getElementById(LANGUAGE_BUTTON_ID);
        if (languageButton) {
            languageButton.textContent = getLanguageLabel();
        }

        window.render?.();
    },

    toggleDebugMode: async () => {
        const result = await Network.toggleDebugMode();
        if (result === null) {
            return;
        }

        const label = i18n.t('debug_mode');
        const subLabel = i18n.t('debug_bytecode_log');
        const stateLabel = result ? i18n.t('enabled') : i18n.t('disabled');
        alert(`${label} (${subLabel}): ${stateLabel}`);
    }
};
