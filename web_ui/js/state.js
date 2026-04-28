/**
 * Central State Management
 * Holds the current game state, configuration, and connectivity flags.
 */
import { getAppBaseUrl, isMulliganPhase } from './constants.js';

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
    roomCode: null,
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
            // Engine may not send mulligan_selection in PlayerDisplay
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

        // Rebuild card index when state updates
        State.rebuildCardIndex();

        // Adaptive Polling: Signal that something actually changed
        State.emit('change-detected');

        // Detect Mulligan Finish
        const isMulliganOld = isMulliganPhase(State.lastPhase);
        const isMulliganNew = isMulliganPhase(newData.phase);

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

            // Engine sends CardDisplay with card_no, name, card_type, orientation
            // Frontend also expects id/card_id for compatibility
            const rawCid = card.id !== undefined ? card.id : card.card_id;
            const cardNo = card.card_no;

            // Enrich with static database data if available (for _img field)
            let enrichedCard = { ...card };
            if (cardNo && State.staticCardDatabase && State.staticCardDatabase[cardNo]) {
                const staticCard = State.staticCardDatabase[cardNo];
                // Merge static data into engine CardDisplay
                enrichedCard = { ...enrichedCard, ...staticCard };
                console.log('[State] Enriched card', cardNo, 'with _img:', staticCard._img);
            } else if (cardNo) {
                console.log('[State] No static data for card_no:', cardNo, 'staticCardDatabase exists:', !!State.staticCardDatabase);
            }

            // Use card_no as the key for engine CardDisplay format
            if (cardNo) {
                const existing = index[cardNo];
                // Only update if we have more data
                if (!existing || (!existing.name && card.name) || (!existing._img && enrichedCard._img)) {
                    index[cardNo] = enrichedCard;
                }
            }

            // Also support numeric IDs for compatibility
            if (rawCid !== undefined && rawCid >= 0) {
                const templateId = rawCid & State.TEMPLATE_MASK;
                if (templateId >= 0) {
                    const existing = index[templateId];
                    const cardText = card.original_text || card.ability_text || card.ability || card.text;
                    const existingText = existing ? (existing.original_text || existing.ability_text || existing.ability || existing.text) : null;

                    if (!existing || (!existingText && cardText) || (!existing.name && card.name) || (!existing._img && enrichedCard._img)) {
                        index[templateId] = { ...enrichedCard, id: templateId };
                    }

                    if (rawCid !== templateId) {
                        index[rawCid] = { ...index[templateId], id: rawCid };
                    }
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
            indexZone(p.waitroom || p.discard);
            indexZone(p.success_live_card_zone);
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
            const base = getAppBaseUrl();
            const withBase = (path) => `${base}${path}`.replace(/\/{2,}/g, '/').replace(':/', '://');
            const fetchOptionalJson = async (path, label) => {
                const response = await fetch(withBase(path));
                if (!response.ok) {
                    console.warn(`[State] Failed to load ${label}:`, response.status);
                    return null;
                }

                const contentType = response.headers.get('content-type') || '';
                if (!contentType.toLowerCase().includes('json')) {
                    console.warn(`[State] Skipping ${label}: expected JSON but got`, contentType || 'unknown content type');
                    return null;
                }

                return response.json();
            };

            // Load cards.json first (most critical)
            const cardsResponse = await fetch(withBase('cards/cards.json'));
            if (!cardsResponse.ok) {
                console.error('[State] Failed to load cards.json:', cardsResponse.status, cardsResponse.statusText);
                // Try fallback path
                const fallbackResponse = await fetch(withBase('./cards/cards.json'));
                if (!fallbackResponse.ok) {
                    console.error('[State] Failed to load fallback cards.json:', fallbackResponse.status);
                    return;
                }
                const cardsData = await fallbackResponse.json();
                State.staticCardDatabase = cardsData;
                console.log('[State] Loaded static card database from fallback, total cards:', Object.keys(cardsData).length);
            } else {
                const cardsData = await cardsResponse.json();
                State.staticCardDatabase = cardsData;
                console.log('[State] Loaded static card database, total cards:', Object.keys(cardsData).length);
            }

            // Verify specific cards that were failing
            const testCards = ['PL!S-bp2-022-L', 'PL!S-PR-025-PR'];
            testCards.forEach(cardNo => {
                if (State.staticCardDatabase[cardNo]) {
                    console.log('[State] Verified card exists:', cardNo, '->', State.staticCardDatabase[cardNo].name);
                } else {
                    console.error('[State] Card still missing:', cardNo);
                }
            });

            // Load card_id_mapping.json
            const mappingData = await fetchOptionalJson('engine/card_id_mapping.json', 'card_id_mapping.json');
            if (mappingData) {
                State.cardIdMapping = mappingData;
                console.log('[State] Loaded card ID mapping, total mappings:', Object.keys(mappingData).length);
            }

            // Load card_image_mapping.json
            const imageMappingData = await fetchOptionalJson('js/card_image_mapping.json', 'card_image_mapping.json');
            if (imageMappingData) {
                State.cardImageMapping = imageMappingData;
                console.log('[State] Loaded card image mapping, total mappings:', Object.keys(imageMappingData).length);
            }
        } catch (e) {
            console.error('[State] Failed to load static card database:', e);
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
        if (cid === null || cid === undefined) return null;

        // Handle string card_no lookups (e.g., "PL!-sd1-001-SD")
        if (typeof cid === 'string') {
            // Try card index first (game-specific data)
            if (State.cardIndex && State.cardIndex[cid]) {
                return State.cardIndex[cid];
            }
            
            // Fallback to static database
            if (State.staticCardDatabase && State.staticCardDatabase[cid]) {
                return State.staticCardDatabase[cid];
            }
            
            // If database not loaded yet, trigger async load and return null
            if (!State.staticCardDatabase) {
                console.warn('[State] Card lookup attempted before database loaded:', cid);
                State.loadStaticCardDatabase();
            }
            
            return null;
        }

        // Handle numeric ID lookups
        if (cid < 0) return null;

        const templateId = cid & State.TEMPLATE_MASK;

        if (!State.data) return null;

        // Use card index if available
        if (State.cardIndex) {
            return State.cardIndex[templateId] || State.cardIndex[cid];
        }

        const state = State.data;
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
                getZoneCards(p.success_live_card_zone)
            ];
            for (const zone of allZones) {
                for (const c of zone) {
                    const card = (typeof c === 'object' && c !== null) ? (c.card || c) : null;
                    if (card && (card.id === cid || card.card_id === cid || card.card_no === cid)) return card;
                }
            }
        }
        if (state.looked_cards) {
            const found = state.looked_cards.find(c => c && (c.id === cid || c.card_id === cid || c.card_no === cid));
            if (found) return found;
        }
        return null;
    },

    resolveCardDataByName: (cardName) => {
        if (!cardName || typeof cardName !== 'string') return null;

        // Search in card index first
        if (State.cardIndex) {
            for (const key in State.cardIndex) {
                const card = State.cardIndex[key];
                if (card && card.name === cardName) {
                    return card;
                }
            }
        }

        // Fallback to static database
        if (State.staticCardDatabase) {
            for (const key in State.staticCardDatabase) {
                const card = State.staticCardDatabase[key];
                if (card && card.name === cardName) {
                    return card;
                }
            }
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
