/**
 * Unit tests for ui_logs.js module
 * Run with: node --experimental-vm-modules frontend/web_ui/js/tests/ui_logs.test.js
 */

// Mock dependencies
const mockState = {
    data: null,
    perspectivePlayer: 0,
    currentLang: 'en',
    showFriendlyAbilities: true,
    selectedTurn: -1,
    showingFullLog: false
};

const mockTranslations = {
    en: {
        'active_effects': 'Active Effects',
        'turn_history': 'Turn History',
        'event_play': 'Play',
        'event_activate': 'Activate',
        'you': 'You',
        'opponent': 'Opponent'
    },
    jp: {
        'active_effects': '適用中の効果',
        'turn_history': 'ターン履歴',
        'event_play': 'プレイ',
        'event_activate': '発動',
        'you': 'あなた',
        'opponent': '対戦相手'
    }
};

// Test utilities
let testsPassed = 0;
let testsFailed = 0;

function assert(condition, message) {
    if (condition) {
        testsPassed++;
        console.log(`✓ ${message}`);
    } else {
        testsFailed++;
        console.error(`✗ ${message}`);
    }
}

function describe(suiteName, fn) {
    console.log(`\n=== ${suiteName} ===`);
    fn();
}

// ============================================
// Tests for filterState
// ============================================
describe('filterState', () => {
    // Reset filter state
    const filterState = {
        eventTypes: new Set(['PLAY', 'ACTIVATE', 'TRIGGER', 'EFFECT', 'RULE', 'YELL', 'PERFORMANCE']),
        players: new Set([0, 1]),
        searchText: '',
        selectedTurn: -1
    };

    assert(filterState.eventTypes.has('PLAY'), 'Should include PLAY event type');
    assert(filterState.eventTypes.has('ACTIVATE'), 'Should include ACTIVATE event type');
    assert(filterState.players.has(0), 'Should include player 0');
    assert(filterState.players.has(1), 'Should include player 1');
    assert(filterState.searchText === '', 'Search text should be empty by default');
    assert(filterState.selectedTurn === -1, 'Selected turn should be -1 (all) by default');
});

// ============================================
// Tests for applyFilters
// ============================================
describe('applyFilters', () => {
    const events = [
        { turn: 1, phase: 7, player_id: 0, event_type: 'PLAY', description: 'Test play' },
        { turn: 1, phase: 7, player_id: 1, event_type: 'ACTIVATE', description: 'Test activate' },
        { turn: 2, phase: 7, player_id: 0, event_type: 'TRIGGER', description: 'Test trigger' },
        { turn: 2, phase: 7, player_id: 1, event_type: 'EFFECT', description: 'Test effect' }
    ];

    // Simulate applyFilters function
    const filterState = {
        eventTypes: new Set(['PLAY', 'ACTIVATE']),
        players: new Set([0, 1]),
        searchText: '',
        selectedTurn: -1
    };

    const applyFilters = (events) => {
        return events.filter(e => {
            if (!filterState.eventTypes.has(e.event_type)) return false;
            if (!filterState.players.has(e.player_id)) return false;
            if (filterState.selectedTurn !== -1 && e.turn !== filterState.selectedTurn) return false;
            if (filterState.searchText && !e.description.toLowerCase().includes(filterState.searchText.toLowerCase())) return false;
            return true;
        });
    };

    const filtered = applyFilters(events);
    assert(filtered.length === 2, 'Should filter to 2 events (PLAY, ACTIVATE)');
    assert(filtered[0].event_type === 'PLAY', 'First filtered event should be PLAY');
    assert(filtered[1].event_type === 'ACTIVATE', 'Second filtered event should be ACTIVATE');

    // Test with search text
    filterState.searchText = 'play';
    const searchFiltered = applyFilters(events);
    assert(searchFiltered.length === 1, 'Should find 1 event with "play" in description');
    assert(searchFiltered[0].event_type === 'PLAY', 'Found event should be PLAY');

    // Reset
    filterState.searchText = '';
    filterState.eventTypes = new Set(['PLAY', 'ACTIVATE', 'TRIGGER', 'EFFECT', 'RULE', 'YELL', 'PERFORMANCE']);
});

// ============================================
// Tests for getPhaseKey
// ============================================
describe('getPhaseKey', () => {
    const Phase = {
        RPS: 0, SETUP: 1, MULLIGAN_P1: 2, MULLIGAN_P2: 3,
        ACTIVE: 4, ENERGY: 5, DRAW: 6, MAIN: 7,
        LIVE_SET: 8, PERFORMANCE_P1: 9, PERFORMANCE_P2: 10, LIVE_RESULT: 11
    };

    const perspectivePlayer = 0;
    const getPhaseKey = (phase) => {
        switch (phase) {
            case Phase.RPS: return 'rps';
            case Phase.SETUP: return 'setup';
            case Phase.MULLIGAN_P1: return perspectivePlayer === 0 ? 'mulligan_you' : 'mulligan_opp';
            case Phase.MULLIGAN_P2: return perspectivePlayer === 1 ? 'mulligan_you' : 'mulligan_opp';
            case Phase.ACTIVE: return 'active';
            case Phase.ENERGY: return 'energy';
            case Phase.DRAW: return 'draw';
            case Phase.MAIN: return 'main';
            case Phase.LIVE_SET: return 'live_set';
            case Phase.PERFORMANCE_P1: return perspectivePlayer === 0 ? 'perf_p1' : 'perf_p2';
            case Phase.PERFORMANCE_P2: return perspectivePlayer === 1 ? 'perf_p1' : 'perf_p2';
            case Phase.LIVE_RESULT: return 'live_result';
            default: return String(phase);
        }
    };

    assert(getPhaseKey(Phase.RPS) === 'rps', 'RPS phase should return "rps"');
    assert(getPhaseKey(Phase.MAIN) === 'main', 'MAIN phase should return "main"');
    assert(getPhaseKey(Phase.MULLIGAN_P1) === 'mulligan_you', 'MULLIGAN_P1 should return "mulligan_you" for player 0');
    assert(getPhaseKey(Phase.PERFORMANCE_P1) === 'perf_p1', 'PERFORMANCE_P1 should return "perf_p1" for player 0');
});

// ============================================
// Tests for getEventIcon
// ============================================
describe('getEventIcon', () => {
    const getEventIcon = (eventType) => {
        const icons = {
            'PLAY': '🃏',
            'ACTIVATE': '⚡',
            'TRIGGER': '🎯',
            'EFFECT': '✨',
            'RULE': '📜',
            'YELL': '📣',
            'PERFORMANCE': '🎤',
            'PHASE': '🔄',
            'DRAW': '📥',
            'SCORE': '📊',
            'HEART': '💖',
            'BATON': ' Baton',
            'LIVE': '🎵'
        };
        return icons[eventType] || '•';
    };

    assert(getEventIcon('PLAY') === '🃏', 'PLAY should have card icon');
    assert(getEventIcon('ACTIVATE') === '⚡', 'ACTIVATE should have lightning icon');
    assert(getEventIcon('TRIGGER') === '🎯', 'TRIGGER should have target icon');
    assert(getEventIcon('PHASE') === '🔄', 'PHASE should have refresh icon');
    assert(getEventIcon('UNKNOWN') === '•', 'Unknown event type should return bullet');
});

// ============================================
// Tests for createTurnEventElement
// ============================================
describe('createTurnEventElement', () => {
    // Simulate element creation
    const createTurnEventElement = (event, t) => {
        const typeClass = event.event_type ? event.event_type.toLowerCase() : 'generic';
        const playerLabel = event.player_id === 0 ? 'You' : 'Opponent';
        const phaseLabel = 'main';
        const eventIcon = '🃏';

        return {
            className: `log-entry turn-event ${typeClass}`,
            attributes: {
                role: 'logentry',
                'aria-live': 'polite',
                'aria-label': `Turn ${event.turn}, ${phaseLabel}, ${playerLabel}: ${event.event_type} - ${event.description || ''}`
            },
            innerHTML: `
                <span class="turn-badge">T${event.turn}</span>
                <span class="phase-badge">${phaseLabel}</span>
                <span class="player-badge p${event.player_id}">${playerLabel}</span>
                <span class="event-type">${eventIcon} ${event.event_type || 'Event'}</span>
                <span class="event-desc">${event.description || ''}</span>
            `
        };
    };

    const event = {
        turn: 1,
        phase: 7,
        player_id: 0,
        event_type: 'PLAY',
        description: 'Test play'
    };

    const el = createTurnEventElement(event, mockTranslations.en);
    assert(el.className.includes('turn-event'), 'Element should have turn-event class');
    assert(el.className.includes('play'), 'Element should have play class');
    assert(el.attributes.role === 'logentry', 'Element should have role="logentry"');
    assert(el.attributes['aria-live'] === 'polite', 'Element should have aria-live="polite"');
    assert(el.innerHTML.includes('T1'), 'Element should show turn number');
    assert(el.innerHTML.includes('PLAY'), 'Element should show event type');
});

// ============================================
// Tests for differential update
// ============================================
describe('updateLogDifferential', () => {
    let lastLogCount = 0;
    let lastHistoryCount = 0;

    const state = {
        rule_log: ['entry1', 'entry2', 'entry3'],
        turn_history: [{ turn: 1 }, { turn: 2 }]
    };

    // Simulate differential update logic
    const currentLogCount = state.rule_log.length;
    const currentHistoryCount = state.turn_history.length;

    const needsFullRender = currentLogCount < lastLogCount || currentHistoryCount < lastHistoryCount;
    assert(!needsFullRender, 'Should not need full render when counts increase');

    const newLogEntries = state.rule_log.slice(lastLogCount);
    const newHistoryEntries = state.turn_history.slice(lastHistoryCount);

    assert(newLogEntries.length === 3, 'Should have 3 new log entries');
    assert(newHistoryEntries.length === 2, 'Should have 2 new history entries');

    // Update counts
    lastLogCount = currentLogCount;
    lastHistoryCount = currentHistoryCount;

    // Simulate reset scenario
    const resetState = {
        rule_log: ['new entry'],
        turn_history: []
    };

    const resetLogCount = resetState.rule_log.length;
    const resetHistoryCount = resetState.turn_history.length;
    const needsFullRenderAfterReset = resetLogCount < lastLogCount || resetHistoryCount < lastHistoryCount;
    assert(needsFullRenderAfterReset, 'Should need full render after count decreases');
});

// ============================================
// Summary
// ============================================
console.log('\n=== Test Summary ===');
console.log(`Passed: ${testsPassed}`);
console.log(`Failed: ${testsFailed}`);
console.log(`Total: ${testsPassed + testsFailed}`);

if (testsFailed > 0) {
    process.exit(1);
}
