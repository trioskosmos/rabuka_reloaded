/**
 * Test script for translateAbility and t() (consistency check)
 */
const path = require('path');
// Note: This script might need 'esm' or similar to run if files use 'import'
// For now, updating the logic to reflect the new API
const { translateAbility, t } = require(path.join(__dirname, 'ability_translator.js'));

function testConsistency() {
    console.log("=== Ability Translation Test ===");
    const raw = "TRIGGER: ON_PLAY\nEFFECT: DRAW(1)";
    console.log("Input:", raw);
    console.log("JP:", translateAbility(raw, 'jp'));
    console.log("EN:", translateAbility(raw, 'en'));

    console.log("\n=== Heuristic Translation Test ===");
    const jpRaw = "自分のデッキの上からカードを3枚見る";
    console.log("Input:", jpRaw);
    console.log("EN (Heuristic):", translateAbility(jpRaw, 'en'));

    console.log("\n=== UI Translation Test (t) ===");
    console.log("Turn (JP):", t('turn'));
    console.log("Pick More (EN, params):", t('pick_more', { count: 3 }));
}

// Mocking window/Translations for the test environment if needed
if (typeof window === 'undefined') {
    global.window = {
        currentTranslationsJP: {},
        currentTranslationsEN: {}
    };
}

try {
    testConsistency();
} catch (e) {
    console.error("Test failed:", e.message);
    console.log("Note: This test script requires an ES module compatible environment.");
}
