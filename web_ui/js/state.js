import { getAppBaseUrl, isMulliganPhase } from './constants.js';

const _target = new EventTarget();

function uiConfig() {
    return State.data?.ui_config || {};
}

const stateInternal = {
    on: (name, cb) => _target.addEventListener(name, cb),
    off: (name, cb) => _target.removeEventListener(name, cb),
    emit: (name, detail) => _target.dispatchEvent(new CustomEvent(name, { detail })),

    data: null,
    rawData: null,
    _forceRender: false,

    roomCode: null,
    sessionToken: null,
    cardSet: 'compiled',
    gameHasStarted: false,

    offlineMode: false,
    isLiveWatchOn: false,

    replayData: null,
    currentFrame: 0,
    playInterval: null,

    lastStateJson: null,
    lastPerformanceData: null,
    lastAssetsHash: null,
    plannerData: null,
    lastPlannerFetchKey: null,
    plannerLoading: false,

    cardIndex: null,
    lastIndexedStateId: null,

    staticCardDatabase: null,
    cardIdMapping: null,

    TEMPLATE_MASK: 0x1FFFFF,
    INSTANCE_SHIFT: 21,

    // Read from Rust ui_config
    get currentLang() { return uiConfig().current_lang || 'jp'; },
    get showFriendlyAbilities() { return uiConfig().show_friendly_abilities || false; },
    get selectedTurn() { return uiConfig().selected_turn ?? -1; },
    get selectedPerfTurn() { return uiConfig().selected_perf_turn ?? -1; },
    get perspectivePlayer() { return uiConfig().perspective_player ?? 0; },
        get replayMode()
 { return uiConfig().replay_mode || false; },

    // Purely rendering state (not in Rust)
    uiMode: 'play', // 'view' | 'play' — controls mobile card interaction
    _choiceModalDismissed: false,
    _choiceStateId: null,
    _sysActionsDismissed: false,
    _aiSessionToken: null,
    _localPerspective: undefined,
    selectedHandIdx: -1,
    showingFullLog: false,
    lastPerformanceTurn: -1,
    deckAnalysis: null,
    fullLogData: null,
    lastActionsHash: null,
    lastShownPerformanceHash: null,
    performanceHistory: {},
    performanceHistoryTurns: [],
    capturedErrors: [],
    hoveredActionId: null,
    localMulliganSelection: new Set(),
    // Maps card_index → button element for mulligan action thumbnails.
    // Populated by ActionButtons.js, consumed by CardRenderer.js.
    mulliganButtons: new Map(),
    localLiveCardSelection: new Set(),
    liveCardButtons: new Map(),
    lastMulliganCards: [],
    showMulliganReturn: false,

    _frameCounter: 0,
    _actionLatency: -1,

    setUiMode: (mode) => {
        if (mode !== 'view' && mode !== 'play') return;
        State.uiMode = mode;
        try { localStorage.setItem('rabuka_ui_mode', mode); } catch (_) {}
        State.emit('ui-mode-change', { mode });
        if (typeof window.render === 'function') window.render();
    },

    toggleUiMode: () => {
        State.setUiMode(State.uiMode === 'view' ? 'play' : 'view');
    },

    updateUiConfig: async (changes) => {
        if (!State.data) State.data = {};
        if (!State.data.ui_config) State.data.ui_config = {};
        Object.assign(State.data.ui_config, changes);

        // Track perspective locally so it survives server round-trips (server value is global)
        if (changes.perspective_player !== undefined) {
            State._localPerspective = changes.perspective_player;
        }

        try {
            const res = await fetch('api/ui/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(changes)
            });
            const data = await res.json();
            if (data.success && data.ui_config && State.data) {
                State.data.ui_config = data.ui_config;
                // Re-apply local perspective after server overwrite
                if (State._localPerspective !== undefined) {
                    State.data.ui_config.perspective_player = State._localPerspective;
                }
            }
            return data;
        } catch (e) {
            console.error('[State] Failed to update UI config:', e);
        }
    },

    update: (newData) => {
        if (!newData) {
            State.data = null;
            State.rawData = null;
            State.cardIndex = null;
            State.lastIndexedStateId = null;
            return;
        }

        if (State.data && newData.state_id !== undefined && newData.state_id === State.data.state_id) {
            State.data.is_ai_thinking = newData.is_ai_thinking;
            State.data.ai_status = newData.ai_status;
            return;
        }

        if (State.data) {
            State.lastPhase = State.data.phase;
            const pOld = State.perspectivePlayer === 0 ? State.data.player1 : State.data.player2;
            if (pOld && pOld.mulligan_selection !== undefined) {
                const selection = pOld.mulligan_selection;
                let indices = [];
                if (Array.isArray(selection)) {
                    indices = selection;
                } else if (typeof selection === 'number') {
                    const handCards = pOld.hand.cards;
                    for (let i = 0; i < handCards.length; i++) {
                        if ((selection >> i) & 1) indices.push(i);
                    }
                }
                if (indices.length > 0) {
                    const handCards = pOld.hand.cards;
                    State.lastMulliganCards = indices.map(idx => handCards[idx]).filter(c => c !== null);
                }
            }
        }

        State.rawData = JSON.parse(JSON.stringify(newData));
        State.data = newData;
        // Preserve local perspective — server's ui_config is global and shared across sessions
        if (newData.ui_config && State._localPerspective !== undefined) {
            State.data.ui_config = State.data.ui_config || {};
            State.data.ui_config.perspective_player = State._localPerspective;
        }
        State.rebuildCardIndex();
        State.emit('change-detected');

        const isMulliganOld = isMulliganPhase(State.lastPhase);
        const isMulliganNew = isMulliganPhase(newData.phase);

        const isLiveCardSetOld = State.lastPhase === 'LiveCardSetFirstAttacker' || State.lastPhase === 'LiveCardSetSecondAttacker';
        const isLiveCardSetNew = newData.phase === 'LiveCardSetFirstAttacker' || newData.phase === 'LiveCardSetSecondAttacker';

        // Clear local selection on any phase change
        if (State.lastPhase !== newData.phase) {
            State.localMulliganSelection.clear();
            State.localLiveCardSelection.clear();
        }

        // During live card set, keep local selection in sync with server
        if (isLiveCardSetOld || isLiveCardSetNew) {
            const pNew = State.perspectivePlayer === 0 ? newData.player1 : newData.player2;
            if (pNew && pNew.live_card_selection) {
                State.localLiveCardSelection.clear();
                const sel = pNew.live_card_selection;
                if (Array.isArray(sel)) {
                    sel.forEach(i => State.localLiveCardSelection.add(i));
                }
            }
        }

        // During mulligan, keep local selection in sync with server
        if (isMulliganOld || isMulliganNew) {
            const pNew = State.perspectivePlayer === 0 ? newData.player1 : newData.player2;
            if (pNew && pNew.mulligan_selection) {
                State.localMulliganSelection.clear();
                const sel = pNew.mulligan_selection;
                if (Array.isArray(sel)) {
                    sel.forEach(i => State.localMulliganSelection.add(i));
                } else if (typeof sel === 'number') {
                    const handCards = pNew.hand.cards;
                    for (let i = 0; i < handCards.length; i++) {
                        if ((sel >> i) & 1) State.localMulliganSelection.add(i);
                    }
                }
            }
        }

        if (isMulliganOld && !isMulliganNew && State.lastMulliganCards.length > 0) {
            State.showMulliganReturn = true;
            setTimeout(() => {
                State.showMulliganReturn = false;
                State.lastMulliganCards = [];
                if (window.render) window.render();
            }, 1000);
        }

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

    rebuildCardIndex: () => {
        const state = State.data;
        if (!state || (!state.player1 && !state.player2)) {
            State.cardIndex = null;
            return;
        }
        const playersList = [state.player1, state.player2];

        const index = {};

        const addCard = (card, zone) => {
            if (!card) return;

            const rawCid = card.id !== undefined ? card.id : card.card_id;
            const cardNo = card.card_no;

            let enrichedCard = { ...card };
            if (cardNo && State.staticCardDatabase && State.staticCardDatabase[cardNo]) {
                const staticCard = State.staticCardDatabase[cardNo];
                enrichedCard = { ...enrichedCard, ...staticCard };
            }

            if (cardNo) {
                const existing = index[cardNo];
                if (!existing || (!existing.name && card.name) || (!existing._img && enrichedCard._img)) {
                    index[cardNo] = enrichedCard;
                }
            }

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

        if (state.master_cards) state.master_cards.forEach(c => addCard(c, 'master'));
        if (state.all_cards) state.all_cards.forEach(c => addCard(c, 'all_cards'));

        playersList.forEach((p, playerIdx) => {
            if (!p) return;

            const indexZone = (zoneData) => {
                if (!zoneData) return;
                const cards = zoneData.cards;
                if (!Array.isArray(cards)) return;
                cards.forEach(c => {
                    if (typeof c === 'number') {
                        addCard({ id: c }, 'zone');
                    } else {
                        addCard(c, 'zone');
                    }
                });
            }

            const indexStage = (stage) => {
                if (!stage) return;
                ['left_side', 'center', 'right_side', 'left_under', 'center_under', 'right_under'].forEach(slot => {
                    const card = stage[slot];
                    if (Array.isArray(card)) {
                        card.forEach(c => addCard(c, 'stage'));
                    } else if (card && typeof card === 'object') {
                        addCard(card, 'stage');
                    }
                });
            }

            indexZone(p.hand);
            indexStage(p.stage);
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
    },

    loadStaticCardDatabase: async () => {
        if (State.staticCardDatabase && State.cardIdMapping) return;

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

            const cardsResponse = await fetch(withBase('cards/cards.json'));
            if (!cardsResponse.ok) {
                console.error('[State] Failed to load cards.json:', cardsResponse.status, cardsResponse.statusText);
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

            const mappingData = await fetchOptionalJson('engine/card_id_mapping.json', 'card_id_mapping.json');
            if (mappingData) {
                State.cardIdMapping = mappingData;
                console.log('[State] Loaded card ID mapping, total mappings:', Object.keys(mappingData).length);
            }

            const imageMappingData = await fetchOptionalJson('js/card_image_mapping.json', 'card_image_mapping.json');
            if (imageMappingData) {
                State.cardImageMapping = imageMappingData;
                console.log('[State] Loaded card image mapping, total mappings:', Object.keys(imageMappingData).length);
                // Replace remote img URLs with local WebP paths in the static database
                if (State.staticCardDatabase) {
                    let replaced = 0;
                    for (const [cardNo, card] of Object.entries(State.staticCardDatabase)) {
                        if (card.img && card.img.startsWith('http')) {
                            const localPath = State.cardImageMapping[cardNo]
                                || State.cardImageMapping[cardNo.normalize('NFKC')];
                            if (localPath) {
                                card.img = localPath;
                                replaced++;
                            } else {
                                delete card.img;
                            }
                        }
                    }
                    if (replaced > 0) console.log('[State] Replaced', replaced, 'remote img URLs with local WebP paths');
                }
                State.emit('carddb-loaded');
                if (State.data) {
                    State.emit('change', State.data);
                }
            }
        } catch (e) {
            console.error('[State] Failed to load static card database:', e);
        }
    },

    initUiMode: () => {
        try {
            const saved = localStorage.getItem('rabuka_ui_mode');
            if (saved === 'view' || saved === 'play') State.uiMode = saved;
        } catch (_) {}
    },

    resetForNewGame: () => {
        State.selectedHandIdx = -1;
        State.lastPerformanceTurn = -1;
        State._sysActionsDismissed = false;
        State._choiceModalDismissed = false;
        State._choiceStateId = null;
        State._aiSessionToken = null;
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
        window.lastShownPerformanceHash = "";
        State._frameCounter = 0;
    },

    resolveCardData: (cid) => {
        if (cid === null || cid === undefined) return null;

        if (typeof cid === 'string') {
            if (State.cardIndex && State.cardIndex[cid]) {
                return State.cardIndex[cid];
            }

            if (State.staticCardDatabase && State.staticCardDatabase[cid]) {
                return State.staticCardDatabase[cid];
            }

            if (!State.staticCardDatabase) {
                console.warn('[State] Card lookup attempted before database loaded:', cid);
                State.loadStaticCardDatabase();
            }

            return null;
        }

        if (cid < 0) return null;

        const templateId = cid & State.TEMPLATE_MASK;

        if (!State.data) return null;

        if (State.cardIndex) {
            const result = State.cardIndex[templateId] || State.cardIndex[cid];
            if (result && result.card_no) return result;
        }

        const state = State.data;
        const playersList = [state.player1, state.player2];
        for (const p of playersList) {
            if (!p) continue;
            const getZoneCards = (zone) => {
                if (!zone) return [];
                if (!Array.isArray(zone.cards)) return [];
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

        if (State.cardIndex) {
            for (const key in State.cardIndex) {
                const card = State.cardIndex[key];
                if (card && card.name === cardName) {
                    return card;
                }
            }
        }

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

    stripRichData: (obj) => {
        if (obj === null || obj === undefined) return obj;

        if (Array.isArray(obj)) {
            return obj.map(item => State.stripRichData(item));
        }

        if (typeof obj === 'object') {
            if ((obj.id !== undefined || obj.card_id !== undefined) && obj.card_no !== undefined) {
                return obj.id !== undefined ? obj.id : obj.card_id;
            }

            const stripped = {};

            const blacklistedKeys = [
                'ai_status', 'is_ai_thinking', 'last_action',
                'mode',
                'my_player_id',
                'needs_deck',
                'spectators',

                'game_over',
                'queue_depth',
                'ui_config'
            ];

            for (const [key, value] of Object.entries(obj)) {
                if (blacklistedKeys.includes(key)) continue;
                stripped[key] = State.stripRichData(value);
            }
            return stripped;
        }

        return obj;
    },

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

if (typeof window !== 'undefined') {
    if (!window.StateMaster) {
        window.StateMaster = stateInternal;
    }
}

export const State = typeof window !== 'undefined' ? window.StateMaster : stateInternal;

export function updateStateData(newData) {
    State.update(newData);
    State._sysActionsDismissed = false;
}

if (typeof window !== 'undefined') {
    window.capturedErrors = State.capturedErrors;
}