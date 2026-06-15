import { State } from '../state.js';
import * as i18n from '../i18n/index.js';

export const SettingsModal = {
    toggleLang: async () => {
        const newLang = State.currentLang === 'jp' ? 'en' : 'jp';
        await State.updateUiConfig({ current_lang: newLang });
        await i18n.loadTranslations(newLang);
        const ui = i18n.getCurrentTranslations()?.ui || {};
        document.querySelectorAll('[data-i18n]').forEach(el => {
            const key = el.getAttribute('data-i18n');
            if (ui[key]) el.textContent = ui[key];
        });
        const btn = document.getElementById('lang-btn');
        if (btn) btn.textContent = newLang === 'jp' ? 'English' : '日本語';
        window.render?.();
    },
};
