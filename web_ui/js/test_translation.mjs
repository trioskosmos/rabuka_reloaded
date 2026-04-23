
import { translateAbility } from './ability_translator.js';

function testConsistency() {
    const raw = "TRIGGER: ON_PLAY\nEFFECT: DRAW(1)";

    console.log("Input Opcode:");
    console.log(raw);

    console.log("\nJapanese Translation:");
    try {
        console.log(translateAbility(raw, 'jp'));
    } catch (e) {
        console.error("JP Translation Error:", e);
    }

    console.log("\nEnglish Translation:");
    try {
        console.log(translateAbility(raw, 'en'));
    } catch (e) {
        console.error("EN Translation Error:", e);
    }
}

testConsistency();
