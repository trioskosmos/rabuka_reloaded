/**
 * i18n/index.js - Entry point for i18n module
 */

export * from './names.js';
export {
    SUPPORTED_LANGS,
    DEFAULT_LANG,
    normalizeLangCode,
    nextLangCode,
    langLabel,
    isDefaultLang,
    isJapanese,
    translateAbility,
    loadTranslations,
    t,
    translateCard,
    translateMetadata,
    translateCardType,
    translateProduct,
    translateSeries,
    translateChoiceDescription,
    getCurrentTranslations,
    getChoicePrompt
} from './translator.js';
