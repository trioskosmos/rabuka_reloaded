// Node.js test for the yell-fallback logic in showRevealedCardsModal
// Tests ONLY the data processing logic (not DOM rendering).

const tests = [];

function assert(condition, msg) {
    if (!condition) throw new Error('ASSERT FAIL: ' + msg);
}

function runTest(name, fn) {
    try {
        fn();
        console.log('PASS:', name);
        tests.push({ name, passed: true });
    } catch (e) {
        console.log('FAIL:', name, '-', e.message);
        tests.push({ name, passed: false });
    }
}

// Simulate the cheer-merge logic exactly as in LogRenderer.js
function simulateCheerMerge(state, perspectivePlayer) {
    const s = state;
    const p1Cheer = (s.player1_cheer_revealed_cards || []).slice().reverse();
    const p2Cheer = (s.player2_cheer_revealed_cards || []).slice().reverse();
    const cheerIds = new Set([
        ...(s.player1_cheer_revealed_cards || []),
        ...(s.player2_cheer_revealed_cards || []),
    ]);

    // ownerOf fallback: scan all zones
    const ownerOf = (cid) => {
        const scan = (pl) => {
            const c = [];
            if (pl.stage) {
                (pl.stage.stage || []).forEach(x => { if (x !== -1) c.push(x); });
            }
            if (pl.hand?.cards) c.push(...pl.hand.cards);
            if (pl.live_zone?.cards) c.push(...pl.live_zone.cards);
            if (pl.success_live_card_zone?.cards) c.push(...pl.success_live_card_zone.cards);
            if (pl.waitroom?.cards) c.push(...pl.waitroom.cards);
            if (pl.energy_zone?.cards) c.push(...pl.energy_zone.cards);
            if (pl.energy_deck?.cards) c.push(...pl.energy_deck.cards);
            if (pl.main_deck?.cards) c.push(...pl.main_deck.cards);
            if (pl.exclusion_zone?.cards) c.push(...pl.exclusion_zone.cards);
            return c;
        };
        const p1s = new Set(scan(s.player1));
        const p2s = new Set(scan(s.player2));
        if (p1s.has(cid)) return 0;
        if (p2s.has(cid)) return 1;
        return -1;
    };

    // THE FIX: Always supplement cheer with persistent yell fields
    const yellIds = new Set([
        ...(s.initial_yell_revealed_cards || []),
        ...(s.re_yell_revealed_cards || []),
    ]);
    [...yellIds].forEach(cid => {
        if (cheerIds.has(cid)) return;
        const owner = ownerOf(cid);
        if (owner === 0) { p1Cheer.unshift(cid); cheerIds.add(cid); }
        else if (owner === 1) { p2Cheer.unshift(cid); cheerIds.add(cid); }
    });

    return { p1Cheer, p2Cheer, cheerIds };
}

// === TEST 1: Re-yell scenario — cheer arrays empty, yell fields populated ===
runTest('re-yell: cheer empty, yell fields populated', () => {
    const state = {
        player1: {
            hand: { cards: [1,2] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [101, 102] },
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player2: {
            hand: { cards: [3,4] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [] },
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player1_cheer_revealed_cards: [],   // cleared by re-yell
        player2_cheer_revealed_cards: [],   // cleared by re-yell
        initial_yell_revealed_cards: [101],  // P1's original yell
        re_yell_revealed_cards: [102],       // P1's re-yell
    };

    const result = simulateCheerMerge(state, 0);

    // p1Cheer should have both yell cards (in newest-first order: 102, 101)
    assert(result.p1Cheer.length === 2, 'Expected 2 cards in p1Cheer, got ' + result.p1Cheer.length);
    assert(result.p1Cheer[0] === 102, 'First card should be 102 (newest), got ' + result.p1Cheer[0]);
    assert(result.p1Cheer[1] === 101, 'Second card should be 101, got ' + result.p1Cheer[1]);
    assert(result.p2Cheer.length === 0, 'Expected 0 cards in p2Cheer, got ' + result.p2Cheer.length);
    assert(result.cheerIds.has(101), 'cheerIds should have 101');
    assert(result.cheerIds.has(102), 'cheerIds should have 102');
});

// === TEST 2: Normal yell — cheer arrays populated, no re-yell ===
runTest('normal yell: cheer arrays populated, yell fields match', () => {
    const state = {
        player1: {
            hand: { cards: [1,2] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [101, 102] },
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player2: {
            hand: { cards: [3,4] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [] },
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player1_cheer_revealed_cards: [101, 102], // already populated
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [101, 102],   // same as cheer
        re_yell_revealed_cards: [],
    };

    const result = simulateCheerMerge(state, 0);

    // Should have same cards, no duplicates
    assert(result.p1Cheer.length === 2, 'Expected 2 cards in p1Cheer, got ' + result.p1Cheer.length);
    assert(result.p1Cheer[0] === 102, 'First should be 102 (newest from reverse), got ' + result.p1Cheer[0]);
    assert(result.p1Cheer[1] === 101, 'Second should be 101, got ' + result.p1Cheer[1]);
});

// === TEST 3: Re-yell on first performance, normal second performance ===
runTest('mixed: re-yell first perf, normal second perf', () => {
    const state = {
        player1: {
            hand: { cards: [1,2] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [101, 102] },
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player2: {
            hand: { cards: [3,4] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [201, 202] },
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player1_cheer_revealed_cards: [],      // cleared by re-yell
        player2_cheer_revealed_cards: [201, 202], // P2's yell from second perf
        initial_yell_revealed_cards: [101, 102],  // P1's yell (persistent)
        re_yell_revealed_cards: [],
    };

    const result = simulateCheerMerge(state, 0);

    // P1's cards from initial_yell_revealed_cards should be in p1Cheer
    assert(result.p1Cheer.length === 2, 'Expected 2 cards in p1Cheer, got ' + result.p1Cheer.length);
    assert(result.p1Cheer.includes(101), 'p1Cheer should have 101');
    assert(result.p1Cheer.includes(102), 'p1Cheer should have 102');
    // P2's cards from cheer_revealed should be in p2Cheer
    assert(result.p2Cheer.length === 2, 'Expected 2 cards in p2Cheer, got ' + result.p2Cheer.length);
    assert(result.p2Cheer.includes(201), 'p2Cheer should have 201');
    assert(result.p2Cheer.includes(202), 'p2Cheer should have 202');
});

// === TEST 4: Unknown owner — card stays in effect (not added to cheer) ===
runTest('unknown owner: card not added to cheer', () => {
    const state = {
        player1: {
            hand: { cards: [1,2] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [] },   // card 999 not here
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player2: {
            hand: { cards: [3,4] },
            stage: { stage: [-1, -1, -1] },
            waitroom: { cards: [] },
            live_zone: { cards: [] },
            success_live_card_zone: { cards: [] },
            energy_zone: { cards: [] },
            energy_deck: { cards: [] },
            main_deck: { cards: [] },
            exclusion_zone: { cards: [] },
        },
        player1_cheer_revealed_cards: [],
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [999], // not in any zone
        re_yell_revealed_cards: [],
    };

    const result = simulateCheerMerge(state, 0);

    // Owner is unknown (-1), should NOT add to cheer
    assert(result.p1Cheer.length === 0, 'p1Cheer should be empty, got ' + result.p1Cheer.length);
    assert(result.p2Cheer.length === 0, 'p2Cheer should be empty, got ' + result.p2Cheer.length);
    // Should NOT be in cheerIds (so it appears in Effect section)
    assert(!result.cheerIds.has(999), 'cheerIds should NOT have 999 (unknown owner stays in Effect)');
});

// === SUMMARY ===
console.log('\n=== RESULTS ===');
const passed = tests.filter(t => t.passed).length;
const failed = tests.filter(t => !t.passed).length;
console.log(`${passed}/${tests.length} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
