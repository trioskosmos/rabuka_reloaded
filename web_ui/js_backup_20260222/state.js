/**
 * Central State Management
 * Holds the current game state, configuration, and connectivity flags.
 */

// We use a mutable object to share state across modules
const stateInternal = {
    // Core Game Data (from backend)
    data: null, // The "state" object from JSON

    // Identity & Session
    roomCode: localStorage.getItem('lovelive_room_code'),
    sessionToken: null,
    perspectivePlayer: 0, // 0 or 1 (Who are we viewing?)

    // Connectivity & Mode
    offlineMode: false,
    wasmAdapter: null,
    hotseatMode: false,
    replayMode: false,
    isLiveWatchOn: false,


    // Replay Data
    replayData: null,
    currentFrame: 0,
    playInterval: null,

    // UI Cache & Optimization
    lastStateJson: null,
    lastPerformanceData: null,
    lastAssetsHash: null,

    // Config
    currentLang: 'jp',
    showFriendlyAbilities: localStorage.getItem('lovelive_friendly_abilities') !== 'false',

    // UI State & Cache
    selectedTurn: -1, // Log selection (-1 means all)
    selectedHandIdx: -1,
    selectedPerfTurn: -1, // Performance result selection (-1 means latest)
    showingFullLog: false,
    lastPerformanceTurn: -1,
    fullLogData: null,
    lastActionsHash: null,
    lastShownPerformanceHash: null,
    performanceHistory: {}, // Stores results by turn number
    performanceHistoryTurns: [], // Sorted list of turns with performance results

    // Error Tracking
    capturedErrors: [],

    // State management helpers
    update: (newData) => {
        if (!newData) {
            State.data = null;
            return;
        }
        State.data = { ...State.data, ...newData };
    },

    resolveCardData: (cid) => {
        if (cid === null || cid === undefined || cid < 0) return null;
        const state = State.data;
        if (state && state.looked_cards) {
            const found = state.looked_cards.find(c => c.id === cid);
            if (found) return found;
        }
        return { id: cid, name: `Card ${cid}`, img: 'icon_blade.png', text: "", original_text: "" };
    },

    resolveCardDataByName: (name) => {
        const state = State.data;
        if (!state) return null;
        for (const p of state.players) {
            if (!p) continue;
            const allZones = [(p.hand || []), (p.stage || []), (p.live_zone || []), (p.energy || []), (p.discard || []), (p.success_lives || p.success_zone || [])];
            for (const zone of allZones) {
                for (const c of zone) {
                    const card = (typeof c === 'object' && c !== null) ? (c.card || c) : null;
                    if (card && card.name === name) return card;
                }
            }
        }
        if (state.looked_cards) {
            const found = state.looked_cards.find(c => c.name === name);
            if (found) return found;
        }
        return null;
    }
};

// Singleton Logic: Ensure all modules share the EXACT same object instance
if (typeof window !== 'undefined') {
    if (!window.StateMaster) {
        window.StateMaster = stateInternal;
    } else {
        // console.warn("[State] Module mismatch potential avoided. Using global StateMaster.");
    }
}

export const State = typeof window !== 'undefined' ? window.StateMaster : stateInternal;

export function updateStateData(newData) {
    State.update(newData);
}
// Global error handler setup (moved here or kept in main, but state tracks errors)
if (typeof window !== 'undefined') {
    window.capturedErrors = State.capturedErrors;
}
