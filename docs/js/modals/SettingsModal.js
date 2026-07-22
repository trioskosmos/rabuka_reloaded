import { State } from '../state.js';
import * as i18n from '../i18n/index.js';

function applyDataI18n() {
    const ui = i18n.getCurrentTranslations()?.ui || {};
    document.querySelectorAll('[data-i18n]').forEach(el => {
        const key = el.getAttribute('data-i18n');
        if (ui[key]) el.innerHTML = ui[key];
    });
}

export const SettingsModal = {
    updateLanguage: () => { applyDataI18n(); },

    toggleLang: async () => {
        const newLang = State.currentLang === 'jp' ? 'en' : 'jp';
        await State.updateUiConfig({ current_lang: newLang });
        await i18n.loadTranslations(newLang);
        applyDataI18n();
        document.querySelectorAll('[data-action="toggle-lang"]').forEach(btn => {
            btn.textContent = newLang === 'jp' ? 'English' : '日本語';
        });
        window.render?.();
    },
};
