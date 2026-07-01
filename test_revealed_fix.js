// Node.js test for the COMPLETELY REWRITTEN showRevealedCardsModal logic
// Tests the flat "collect all IDs from all sources" approach.

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

// Simulate the NEW flat-collection logic from showRevealedCardsModal
function simulateFlatCollect(state) {
    const s = state;
    const allIds = new Set();

    // 1. Cheer / yell (including persistent fields)
    for (const src of ['player1_cheer_revealed_cards', 'player2_cheer_revealed_cards',
                       'initial_yell_revealed_cards', 're_yell_revealed_cards',
                       'revealed_cost_cards']) {
        (s[src] || []).forEach(id => allIds.add(id));
    }

    // 2. Effect reveals
    if (s.revealed_card_info?.length) {
        s.revealed_card_info.forEach(e => { if (e.card_id !== undefined) allIds.add(e.card_id); });
    } else {
        (s.revealed_cards || []).forEach(id => allIds.add(id));
    }
    if (s.revealed_cost_card_info?.length) {
        s.revealed_cost_card_info.forEach(e => { if (e.card_id !== undefined) allIds.add(e.card_id); });
    }

    // 3. Looked cards
    (s.looked_cards?.cards || []).forEach(c => {
        const id = typeof c === 'number' ? c : (c.card_id ?? c.id);
        if (id !== undefined) allIds.add(id);
    });

    return [...allIds].filter(id => id > 0);
}

// === TEST 1: Normal yell ===
runTest('normal yell: cheer arrays populated', () => {
    const state = {
        player1_cheer_revealed_cards: [101, 102],
        player2_cheer_revealed_cards: [201, 202],
        initial_yell_revealed_cards: [],
        re_yell_revealed_cards: [],
        revealed_cost_cards: [],
        revealed_cards: [101, 102, 201, 202],
        revealed_card_info: null,
        revealed_cost_card_info: null,
        looked_cards: null,
    };
    const ids = simulateFlatCollect(state);
    assert(ids.length === 4, 'Expected 4 IDs, got ' + ids.length);
    assert(ids.includes(101), 'Missing 101');
    assert(ids.includes(102), 'Missing 102');
    assert(ids.includes(201), 'Missing 201');
    assert(ids.includes(202), 'Missing 202');
});

// === TEST 2: Re-yell scenario (cheer empty, yell fields populated) ===
runTest('re-yell: cheer empty, yell fields populated', () => {
    const state = {
        player1_cheer_revealed_cards: [],
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [101, 102],
        re_yell_revealed_cards: [103, 104],
        revealed_cost_cards: [],
        revealed_cards: [101, 102, 103, 104],
        revealed_card_info: null,
        revealed_cost_card_info: null,
        looked_cards: null,
    };
    const ids = simulateFlatCollect(state);
    assert(ids.length === 4, 'Expected 4 IDs, got ' + ids.length);
    assert(ids.includes(101), 'Missing 101');
    assert(ids.includes(103), 'Missing 103');
});

// === TEST 3: revealed_card_info objects ===
runTest('revealed_card_info objects with card_id', () => {
    const state = {
        player1_cheer_revealed_cards: [],
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [],
        re_yell_revealed_cards: [],
        revealed_cost_cards: [],
        revealed_cards: [101],
        revealed_card_info: [{ card_id: 101, owner: 0 }, { card_id: 102, owner: 1 }],
        revealed_cost_card_info: null,
        looked_cards: null,
    };
    const ids = simulateFlatCollect(state);
    assert(ids.length === 2, 'Expected 2 IDs from revealed_card_info, got ' + ids.length);
    assert(ids.includes(101), 'Missing 101');
    assert(ids.includes(102), 'Missing 102');
});

// === TEST 4: Looked cards ===
runTest('looked cards with card_id', () => {
    const state = {
        player1_cheer_revealed_cards: [],
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [],
        re_yell_revealed_cards: [],
        revealed_cost_cards: [],
        revealed_cards: [],
        revealed_card_info: null,
        revealed_cost_card_info: null,
        looked_cards: { cards: [{ card_id: 101, id: 101 }, { card_id: 102, id: 102 }] },
    };
    const ids = simulateFlatCollect(state);
    assert(ids.length === 2, 'Expected 2 looked card IDs, got ' + ids.length);
});

// === TEST 5: Deduplication ===
runTest('deduplication across sources', () => {
    const state = {
        player1_cheer_revealed_cards: [101],
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [101],  // same card
        re_yell_revealed_cards: [],
        revealed_cost_cards: [101],          // same card
        revealed_cards: [101],
        revealed_card_info: null,
        revealed_cost_card_info: null,
        looked_cards: null,
    };
    const ids = simulateFlatCollect(state);
    assert(ids.length === 1, 'Expected 1 ID (deduped), got ' + ids.length);
    assert(ids[0] === 101, 'Expected 101');
});

// === TEST 6: Negative IDs filtered out ===
runTest('negative IDs filtered out', () => {
    const state = {
        player1_cheer_revealed_cards: [-1, -2, 101],
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [],
        re_yell_revealed_cards: [],
        revealed_cost_cards: [],
        revealed_cards: [],
        revealed_card_info: null,
        revealed_cost_card_info: null,
        looked_cards: null,
    };
    const ids = simulateFlatCollect(state);
    assert(ids.length === 1, 'Expected 1 positive ID, got ' + ids.length);
    assert(ids[0] === 101, 'Expected 101');
});

// === TEST 7: Empty state ===
runTest('empty state returns empty array', () => {
    const state = {
        player1_cheer_revealed_cards: [],
        player2_cheer_revealed_cards: [],
        initial_yell_revealed_cards: [],
        re_yell_revealed_cards: [],
        revealed_cost_cards: [],
        revealed_cards: [],
        revealed_card_info: null,
        revealed_cost_card_info: null,
        looked_cards: null,
    };
    const ids = simulateFlatCollect(state);
    assert(ids.length === 0, 'Expected empty array, got ' + ids.length);
});

// === SUMMARY ===
console.log('\n=== RESULTS ===');
const passed = tests.filter(t => t.passed).length;
const failed = tests.filter(t => !t.passed).length;
console.log(`${passed}/${tests.length} passed, ${failed} failed`);
if (failed > 0) process.exit(1);
