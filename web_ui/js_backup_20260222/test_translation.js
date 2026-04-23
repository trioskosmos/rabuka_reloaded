/**
 * Test script for translateAbility (consistency check)
 */
// Use absolute path for safety in this environment
const path = require('path');
const { translateAbility } = require(path.join(__dirname, 'ability_translator.js'));

function testConsistency() {
    const raw = "TRIGGER: ON_PLAY\nEFFECT: DRAW(1)";

    console.log("Input Opcode:");
    console.log(raw);

    console.log("\nJapanese Translation:");
    console.log(translateAbility(raw, 'jp'));

    console.log("\nEnglish Translation:");
    console.log(translateAbility(raw, 'en'));
}

testConsistency();
