/**
 * Central State Management
 * Holds the current game state, configuration, and connectivity flags.
 */

const _target = new EventTarget();

// We use a mutable object to share state across modules
const stateInternal = {
    // Event System
    on: (name, cb) => _target.addEventListener(name, cb),
    off: (name, cb) => _target.removeEventListener(name, cb),
    emit: (name, detail) => _target.dispatchEvent(new CustomEvent(name, { detail })),

    // Core Game Data (from backend)
    data: null, // The "state" object from JSON (may be enriched with objects)
    rawData: null, // The "original" state object from server (IDs only, for debugging/warping)

    // Identity & Session
    roomCode: localStorage.getItem('lovelive_room_code'),
    sessionToken: null,
    perspectivePlayer: 0, // 0 or 1 (Who are we viewing?)
    cardSet: 'compiled', // 'compiled' or 'vanilla'
    gameHasStarted: false, // Track if we've moved past Setup phase (prevents deck modal from showing during gameplay)

    // Connectivity & Mode
    offlineMode: false, // Using Rust backend via Express proxy, not WASM
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
    plannerData: null,
    lastPlannerFetchKey: null,
    plannerLoading: false,

    // Card ID Index for O(1) lookups (performance optimization)
    cardIndex: null,
    lastIndexedStateId: null,

    // Static card database from cards.json
    staticCardDatabase: null,

    // Card ID mapping from engine/card_id_mapping.json (numeric ID -> string card_no)
    cardIdMapping: null,

    // Config
    currentLang: localStorage.getItem('lovelive_lang') || 'jp',
    showFriendlyAbilities: localStorage.getItem('lovelive_friendly_abilities') === 'true', // Defaults to false if not set to 'true'

    // Card ID Constants (Must match Rust engine)
    TEMPLATE_MASK: 0x1FFFFF, // Bits 0-20
    INSTANCE_SHIFT: 21,      // Bits 21-30 are UID

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

    // Hover Tracking (Persistence across re-renders)
    hoveredActionId: null,

    // Mulligan Tracking (for visual movement to bottom)
    localMulliganSelection: new Set(),
    lastMulliganCards: [],
    showMulliganReturn: false,

    update: (newData) => {
        if (!newData) {
            State.data = null;
            State.rawData = null;
            State.cardIndex = null;
            State.lastIndexedStateId = null;
            return;
        }

        // Optimization: Skip if state ID hasn't changed
        if (State.data && newData.state_id !== undefined && newData.state_id === State.data.state_id) {
            // Even if state_id is same, we might want to sync connectivity flags which are UI-side
            State.data.is_ai_thinking = newData.is_ai_thinking;
            State.data.ai_status = newData.ai_status;
            return;
        }

        if (State.data) {
            State.lastPhase = State.data.phase;
            const pOld = State.perspectivePlayer === 0 ? State.data.player1 : State.data.player2;
            if (pOld && pOld.mulligan_selection !== undefined) {
                // If it's a number (bitmask), we need to extract indices
                const selection = pOld.mulligan_selection;
                let indices = [];
                if (Array.isArray(selection)) {
                    indices = selection;
                } else if (typeof selection === 'number') {
                    // Rust backend: hand is { cards: [...] }
                    const handCards = pOld.hand.cards;
                    for (let i = 0; i < handCards.length; i++) {
                        if ((selection >> i) & 1) indices.push(i);
                    }
                }
                if (indices.length > 0) {
                    // Rust backend: hand is { cards: [...] }
                    const handCards = pOld.hand.cards;
                    State.lastMulliganCards = indices.map(idx => handCards[idx]).filter(c => c !== null);
                }
            }
        }

        // Deep clone for rawData to ensure we have a pure ID-only version
        // Only if newData is from server (non-circular)
        State.rawData = JSON.parse(JSON.stringify(newData));
        State.data = newData;  // Replace entirely instead of merging
        
        // Debug: Log the phase received from engine
        if (newData.phase) {
            console.log('Received phase from engine:', newData.phase, typeof newData.phase);
        }
        
        // Rebuild card index when state updates
        State.rebuildCardIndex();

        // Adaptive Polling: Signal that something actually changed
        State.emit('change-detected');

        // Detect Mulligan Finish
        const isMulliganOld = State.lastPhase === 'Mulligan';
        const isMulliganNew = newData.phase === 'Mulligan';

        // Clear local selection on phase change
        if (State.lastPhase !== newData.phase) {
            State.localMulliganSelection.clear();
        }

        if (isMulliganOld && !isMulliganNew && State.lastMulliganCards.length > 0) {
            // Transitioned out of mulligan
            State.showMulliganReturn = true;
            setTimeout(() => {
                State.showMulliganReturn = false;
                State.lastMulliganCards = [];
                if (window.render) window.render();
            }, 1000); // 1s is enough for mental bridge
        }

        // Sync performance history on every state update
        if (newData.performance_history && Array.isArray(newData.performance_history)) {
            newData.performance_history.forEach(item => {
                const t = item.turn;
                const p = item.player_id;
                if (t !== undefined && p !== undefined) {
                    if (!State.performanceHistory[t]) State.performanceHistory[t] = {};
                    State.performanceHistory[t][p] = item;
                    if (!State.performanceHistoryTurns.includes(t)) {
                        State.performanceHistoryTurns.push(t);
                    }
                }
            });
            State.performanceHistoryTurns.sort((a, b) => b - a);
        }
        State.emit('change', State.data);
    },

    /**
     * Rebuilds the card ID index for O(1) lookups.
     * Called automatically on state update.
     */
    rebuildCardIndex: () => {
        const state = State.data;
        const playersList = [state.player1, state.player2];
        if (!state || (!state.player1 && !state.player2)) {
            State.cardIndex = null;
            return;
        }

        const index = {};

        // Helper to add cards to index
        const addCard = (card, zone) => {
            if (!card) return;
            // Support both 'id' (client/runtime) and 'card_id' (server/master data)
            const rawCid = card.id !== undefined ? card.id : card.card_id;
            if (rawCid === undefined || rawCid < 0) return;

            // Mask the ID to find the template
            const templateId = rawCid & State.TEMPLATE_MASK;

            if (templateId >= 0) {
                // Store first occurrence OR update if this one has more data (name, text)
                // We use the templateId as the key for metadata resolution
                const existing = index[templateId];
                const cardText = card.original_text || card.ability_text || card.ability || card.text;
                const existingText = existing ? (existing.original_text || existing.ability_text || existing.ability || existing.text) : null;

                if (!existing || (!existingText && cardText) || (!existing.name && card.name)) {
                    index[templateId] = { ...card, id: templateId };
                }

                // Also store the packed version if it's different and we are in a dynamic zone
                if (rawCid !== templateId) {
                    index[rawCid] = { ...index[templateId], id: rawCid };
                }
            }
        };

        // 1. Index master data first (baseline)
        if (state.master_cards) state.master_cards.forEach(c => addCard(c, 'master'));
        if (state.all_cards) state.all_cards.forEach(c => addCard(c, 'all_cards'));

        // Index player zones (using playersList already defined above)
        playersList.forEach((p, playerIdx) => {
            if (!p) return;

            const indexZone = (zoneData) => {
                if (!zoneData) return;
                const cards = zoneData.cards;
                if (!Array.isArray(cards)) return;
                cards.forEach(c => {
                    if (typeof c === 'number') {
                        // Create a skeleton for the ID so addCard can enrich it from index[templateId]
                        addCard({ id: c }, 'zone');
                    } else {
                        addCard(c, 'zone');
                    }
                });
            }

            indexZone(p.hand);
            indexZone(p.stage);
            indexZone(p.live_zone);
            indexZone(p.looked_cards);
            if (p.energy) {
                const energyCards = p.energy.cards;
                indexZone(energyCards.map(e => (e && e.card) ? e.card : e));
            }
            indexZone(p.discard);
            indexZone(p.success_lives || p.success_zone || p.success_pile);
        });

        State.cardIndex = index;
        console.log(`[State] Card index rebuilt. Size: ${Object.keys(index).length}`);
    },

    /**
     * Loads static card database from cards.json
     */
    loadStaticCardDatabase: async () => {
        if (State.staticCardDatabase && State.cardIdMapping) return; // Already loaded

        try {
            // Load card_id_mapping.json
            const mappingResponse = await fetch('/engine/card_id_mapping.json');
            if (mappingResponse.ok) {
                const mappingData = await mappingResponse.json();
                State.cardIdMapping = mappingData;
                console.log('[State] Loaded card ID mapping, total mappings:', Object.keys(mappingData).length);
            } else {
                console.warn('[State] Failed to load card_id_mapping.json:', mappingResponse.status);
            }

            // Load cards.json
            const cardsResponse = await fetch('/cards/cards.json');
            if (!cardsResponse.ok) {
                console.warn('[State] Failed to load cards.json:', cardsResponse.status);
                return;
            }
            const cardsData = await cardsResponse.json();
            State.staticCardDatabase = cardsData;
            console.log('[State] Loaded static card database, total cards:', Object.keys(cardsData).length);
        } catch (e) {
            console.warn('[State] Failed to load static card database:', e);
        }
    },

    /**
     * Resets game-specific state when joining a new room or starting a new game.
     * This prevents old performance data from leaking into new games.
     */
    resetForNewGame: () => {
        State.selectedTurn = -1;
        State.selectedHandIdx = -1;
        State.selectedPerfTurn = -1;
        State.lastPerformanceTurn = -1;
        State.showingFullLog = false;
        State.fullLogData = null;
        State.lastActionsHash = null;
        State.lastShownPerformanceHash = null;
        State.performanceHistory = {};
        State.performanceHistoryTurns = [];
        State.gameHasStarted = false;
        State.lastPerformanceData = null;
        State.lastStateJson = null;
        State.lastAssetsHash = null;
        State.plannerData = null;
        State.lastPlannerFetchKey = null;
        State.perspectivePlayer = 0;
        window.lastShownPerformanceHash = "";
    },

    resolveCardData: (cid) => {
        if (cid === null || cid === undefined || cid < 0) return null;

        const templateId = cid & State.TEMPLATE_MASK;

        if (State.cardIndex) {
            if (State.cardIndex[cid]) return State.cardIndex[cid];
            if (State.cardIndex[templateId]) return { ...State.cardIndex[templateId], id: cid };
        }

        if (State.cardIdMapping && State.staticCardDatabase) {
            const cardNoString = Object.keys(State.cardIdMapping).find(key => State.cardIdMapping[key] === cid);
            if (cardNoString && State.staticCardDatabase[cardNoString]) {
                const card = State.staticCardDatabase[cardNoString];
                const convertedCard = { ...card, id: cid, card_no: cardNoString };
                if (card._img) {
                    const match = card._img.match(/([^\/]+)\.(png|jpg|jpeg|webp)$/i);
                    if (match) convertedCard._img = `img/cards_webp/${match[1]}.webp`;
                } else {
                    convertedCard._img = `img/cards_webp/${cardNoString}.webp`;
                }
                return convertedCard;
            }

            const templateCardNoString = Object.keys(State.cardIdMapping).find(key => State.cardIdMapping[key] === templateId);
            if (templateCardNoString && State.staticCardDatabase[templateCardNoString]) {
                const card = State.staticCardDatabase[templateCardNoString];
                const convertedCard = { ...card, id: cid, card_no: templateCardNoString };
                if (card._img) {
                    const match = card._img.match(/([^\/]+)\.(png|jpg|jpeg|webp)$/i);
                    if (match) convertedCard._img = `img/cards_webp/${match[1]}.webp`;
                } else {
                    convertedCard._img = `img/cards_webp/${templateCardNoString}.webp`;
                }
                return convertedCard;
            }
        }

        return { id: cid, name: `Card ${templateId}`, _img: `img/cards_webp/${templateId}.webp`, text: "", original_text: "" };
    },

    resolveCardDataByName: (name) => {
        const state = State.data;
        if (!state) return null;

        // Use card index if available
        if (State.cardIndex) {
            for (const card of Object.values(State.cardIndex)) {
                if (card && card.name === name) return card;
            }
        }

        const playersList = [state.player1, state.player2];
        for (const p of playersList) {
            if (!p) continue;
            const getZoneCards = (zone) => {
                if (!zone) return [];
                return zone.cards;
            };
            const allZones = [
                getZoneCards(p.hand),
                getZoneCards(p.stage),
                getZoneCards(p.live_zone),
                getZoneCards(p.energy),
                getZoneCards(p.waitroom || p.discard),
                getZoneCards(p.success_live_card_zone || p.success_lives || p.success_zone || p.success_pile)
            ];
            for (const zone of allZones) {
                for (const c of zone) {
                    const card = (typeof c === 'object' && c !== null) ? (c.card || c) : null;
                    if (card && card.name === name) return card;
                }
            }
        }
        if (state.looked_cards) {
            const found = state.looked_cards.find(c => c && c.name === name);
            if (found) return found;
        }
        return null;
    },

    /**
     * Traverses the state object and converts "Rich Card" objects back into
     * simple integer IDs (card_id) that the Rust engine expects for deserialization.
     * Also maps frontend-specific keys (like active_player) back to engine-specific keys (current_player).
     * Preserves history data for undo/redo compatibility.
     */
    stripRichData: (obj) => {
        if (obj === null || obj === undefined) return obj;

        if (Array.isArray(obj)) {
            return obj.map(item => State.stripRichData(item));
        }

        if (typeof obj === 'object') {
            // 1. Handle Card Objects: If this has id/card_id and card_no, it's a rich card
            if ((obj.id !== undefined || obj.card_id !== undefined) && obj.card_no !== undefined) {
                return obj.id !== undefined ? obj.id : obj.card_id;
            }

            // 2. Regular object: recurse but purge UI-only fields
            const stripped = {};

            // Blacklist only UI-specific fields, preserve gameplay state and history
            const blacklistedKeys = [
                'ai_status', 'is_ai_thinking', 'last_action',
                'mode',  // UI mode, not game state
                'my_player_id',  // Frontend viewer perspective
                'needs_deck',  // UI state
                'spectators',  // Server metadata
                'triggered_abilities', 'opponent_triggered_abilities',  // UI renderings
                'game_over',  // Derivable from game state
                'queue_depth'  // UI state
                // NOTE: Preserve 'winner' for proper state restoration
                // NOTE: Preserve 'undo_stack', 'redo_stack' if they exist for history replay
            ];

            // Map keys for engine compatibility without mutating the source object.
            if (obj.active_player !== undefined && obj.current_player === undefined) {
                stripped.current_player = obj.active_player;
            }

            for (const [key, value] of Object.entries(obj)) {
                if (blacklistedKeys.includes(key)) continue;
                stripped[key] = State.stripRichData(value);
            }
            return stripped;
        }

        return obj;
    },

    /**
     * Produces a compact, editable checkpoint payload for the debug tools.
     * This keeps mutable game state while dropping static card catalogs and
     * heavyweight derived debug data that the backend can reconstruct.
     * KEEPS: bytecode_log, rule_log (needed for backend deserialization and debugging)
     */
    createCheckpointData: (obj = null) => {
        const baseSource = obj ?? State.rawData ?? State.data;
        if (baseSource === null || baseSource === undefined) return baseSource;

        if (typeof baseSource === 'object' && !Array.isArray(baseSource)) {
            if (baseSource.raw_state && typeof baseSource.raw_state === 'object') {
                return JSON.parse(JSON.stringify(baseSource.raw_state));
            }
            if (baseSource.checkpoint_state && typeof baseSource.checkpoint_state === 'object') {
                return JSON.parse(JSON.stringify(baseSource.checkpoint_state));
            }
        }

        const clonedSource = (typeof baseSource === 'object')
            ? JSON.parse(JSON.stringify(baseSource))
            : baseSource;
        const checkpoint = State.stripRichData(clonedSource);

        if (!checkpoint || typeof checkpoint !== 'object' || Array.isArray(checkpoint)) {
            return checkpoint;
        }

        const removableTopLevelKeys = [
            'master_cards',
            'all_cards',
            'legal_actions',
            'performance_history',
            'performance_history_turns',
            'action_log',
            'full_log',
            'turn_log'
        ];

        removableTopLevelKeys.forEach((key) => {
            delete checkpoint[key];
        });

        return checkpoint;
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

/**
 * Update state with Rust backend data directly - no transformation
 * Rust: { turn, phase, player1, player2 }
 */
export function updateStateData(newData) {
    State.update(newData);
}
// Global error handler setup (moved here or kept in main, but state tracks errors)
if (typeof window !== 'undefined') {
    window.capturedErrors = State.capturedErrors;
}
