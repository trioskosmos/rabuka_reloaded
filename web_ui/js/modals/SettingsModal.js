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
        // Cycle through the registry (today: jp ↔ en). Adding a language
        // needs no change here.
        const newLang = i18n.nextLangCode(State.currentLang);
        await State.updateUiConfig({ current_lang: newLang });
        await i18n.loadTranslations(newLang);
        applyDataI18n();
        // Button shows the *next* language's autonym (what you'll switch to).
        const following = i18n.langLabel(i18n.nextLangCode(newLang));
        document.querySelectorAll('[data-action="toggle-lang"]').forEach(btn => {
            btn.textContent = following;
        });
        window.render?.();
    },
};
