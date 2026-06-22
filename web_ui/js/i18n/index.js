/**
 * i18n/index.js - Entry point for i18n module
 */

export * from './names.js';
export { 
    translateAbility, 
    loadTranslations, 
    t, 
    translateCard, 
    translateMetadata,
    translateCardType,
    translateProduct,
    translateSeries,
    translateChoiceDescription,
    getCurrentTranslations
} from './translator.js';
