
import init, { WasmEngine } from '../pkg/engine_rust.js';
import { ActionBases } from './generated_constants.js';
import { wasmLoader } from './wasm_loader.js';
import { State } from './state.js';

export class WasmAdapter {
    constructor() {
        this.engine = null;
        this.initialized = false;
        this.cardDbRaw = null;
        this.cardDb = null;
        this.initPromise = null;
    }

    async init() {
        if (this.initialized) return;
        if (this.initPromise) return this.initPromise;

        this.initPromise = (async () => {
            console.log("[WASM] Initializing...");
            try {
                await init();
                console.log("[WASM] Loaded.");

                // Load Card DB - use cards.json from the cards directory
                const res = await fetch('/cards/cards.json');
                if (!res.ok) {
                    throw new Error(`Failed to load cards.json: ${res.status}`);
                }
                const cardsData = await res.json();
                this.cardDb = cardsData; // Keep a JS copy for lookups

                // WASM engine expects a different format - need to convert
                // For now, use empty JSON as placeholder since WASM mode may not be fully functional
                this.cardDbRaw = '{}';

                this.engine = new WasmEngine(this.cardDbRaw);
                this.initialized = true;
                console.log("[WASM] Engine Ready.");

                // Create a default game state
                this.createOfflineGame();

            } catch (e) {
                console.error("[WASM] Init failed:", e);
                throw e;
            }
        })();
        return this.initPromise;
    }

    createOfflineGame() {
        // Default init with blank boards
        this.engine.init_game(
            new Uint32Array([]), new Uint32Array([]),
            new Uint32Array([]), new Uint32Array([]),
            new Uint32Array([]), new Uint32Array([]),
            BigInt(Date.now())
        );
    }

    createGameWithDecks(p0, p1) {
        if (!this.engine) return { success: false, error: "Engine not initialized" };

        console.log("[WASM] Init game with decks:", p0, p1);

        this.engine.init_game(
            new Uint32Array(p0.deck || []), new Uint32Array(p1.deck || []),
            new Uint32Array(p0.energy || []), new Uint32Array(p1.energy || []),
            new Uint32Array(p0.lives || []), new Uint32Array(p1.lives || []),
            BigInt(Date.now())
        );
        return { success: true };
    }

    // --- API Replacements ---

    async fetchState() {
        if (!this.initialized) await this.init();

        try {
            const json = this.engine.get_state_json();
            const state = JSON.parse(json);

            // Augment state
            state.mode = "pve";
            state.is_pvp = false;
            state.my_player_id = 0;

            // Generate enriched legal actions
            state.legal_actions = this.enrichLegalActions(state);

            return { success: true, state: state };
        } catch (e) {
            console.error(e);
            return { success: false, error: e.toString() };
        }
    }

    async doAction(actionId) {
        if (!this.initialized) return { success: false, error: "Not initialized" };
        try {
            this.engine.step(actionId);
            return await this.fetchState();
        } catch (e) {
            return { success: false, error: e.toString() };
        }
    }

    async resetGame() {
        if (!this.initialized) return;
        // Reuse current decks if possible, or clear?
        // In Python reset uses stored decks.
        // We should store decks in this adapter.
        if (this.lastDecks) {
            this.engine.init_game(
                new Uint32Array(this.lastDecks.p0.deck), new Uint32Array(this.lastDecks.p1.deck),
                new Uint32Array(this.lastDecks.p0.energy), new Uint32Array(this.lastDecks.p1.energy),
                new Uint32Array(this.lastDecks.p0.lives), new Uint32Array(this.lastDecks.p1.lives),
                BigInt(Date.now())
            );
        } else {
            this.createOfflineGame();
        }
        return await this.fetchState();
    }

    async aiSuggest(sims) {
        if (!this.initialized) return { success: false };
        try {
            const actionId = this.engine.ai_suggest(sims || 500);

            // Map ID to description for UI
            const enriched = this.enrichAction(actionId, this.getLastState());
            const suggestions = [{
                action_id: actionId,
                desc: enriched.desc || ("Action " + actionId),
                value: 0.5, // Dummy value
                visits: sims
            }];
            return { success: true, suggestions: suggestions };
        } catch (e) {
            return { success: false, error: e.toString() };
        }
    }

    // --- Deck Management ---

    async uploadDeck(playerId, content) {
        // content is either raw HTML or JSON list of IDs
        let deckList = [];
        try {
            // Try JSON first
            deckList = JSON.parse(content);
        } catch {
            // Parse HTML (Deck Log)
            deckList = this.parseDeckLogHtml(content);
        }

        if (!deckList || deckList.length === 0) return { success: false, error: "Invalid deck content" };

        const config = this.resolveDeckList(deckList);

        if (!this.lastDecks) this.lastDecks = { p0: { deck: [], energy: [], lives: [] }, p1: { deck: [], energy: [], lives: [] } };
        this.lastDecks[playerId === 0 ? 'p0' : 'p1'] = config;

        // Re-init game with new decks
        this.createGameWithDecks(this.lastDecks.p0, this.lastDecks.p1);

        return { success: true, message: `Loaded ${config.deck.length} members, ${config.lives.length} lives, ${config.energy.length} energy.` };
    }

    async loadNamedDeck(deckName) {
        try {
            // Try relative path first (GitHub Pages / Static)
            const res = await fetch(`decks/${deckName}.txt`);
            if (!res.ok) throw new Error(`Status ${res.status}`);
            const text = await res.text();

            // Extract PL! IDs (simple regex parsing for the txt format)
            const matches = text.match(/(PL![A-Za-z0-9\-]+)/g);
            if (!matches) throw new Error("No card IDs found");

            return this.resolveDeckList(matches);
        } catch (e) {
            console.error(`Failed to load named deck ${deckName}:`, e);
            return null;
        }
    }

    async resolveDeckList(deckList) {
        if (!deckList || !Array.isArray(deckList)) return { deck: [], energy: [], lives: [] };

        const deck = [];
        const energy = [];
        const lives = [];

        deckList.forEach(rawId => {
            // Resolve card using State.resolveCardData
            const card = State.resolveCardData(rawId);
            if (card) {
                const id = card.card_no || rawId;
                // Determine card type from card data
                const cardType = card.card_type || '';
                if (cardType === 'Energy' || cardType === 'エネルギー' || String(id).startsWith('LL-E')) {
                    energy.push(id);
                } else if (cardType === 'Live' || cardType === 'ライブ' || card.score !== undefined) {
                    lives.push(id);
                } else {
                    deck.push(id);
                }
            }
        });

        return { deck, energy, lives };
    }

    parseDeckLogHtml(html) {
        const regex = /title="([^"]+?) :[^"]*"[^>]*>.*?class="num">(\d+)<\/span>/gs;
        const cards = [];
        let match;
        while ((match = regex.exec(html)) !== null) {
            const cardNo = match[1].trim();
            const qty = parseInt(match[2], 10);
            for (let i = 0; i < qty; i++) cards.push(cardNo);
        }
        return cards;
    }

    // --- Helpers ---

    getLastState() {
        // Helper to get state without parsing everything if possible,
        // but we need it for context.
        return JSON.parse(this.engine.get_state_json());
    }

    enrichLegalActions(state) {
        const rawIds = this.engine.get_legal_actions(); // Uint32Array
        console.log('[WASM] Raw action IDs:', Array.from(rawIds));
        const actions = Array.from(rawIds).map(id => this.enrichAction(id, state)).filter(a => a !== null);
        console.log('[WASM] Enriched actions:', actions);

        // Don't deduplicate - each action ID encodes the specific stage area
        // The ActionListView will group them by hand_idx for display
        console.log('[WASM] Final actions:', actions);
        return actions;
    }

    enrichAction(id, state) {
        // Logic to reverse-engineer action details from ID and State
        // Rust backend format: player1, player2
        const currentPlayer = state.active_player ?? 0;
        const p = currentPlayer === 0 ? state.player1 : state.player2;

        if (id === ActionBases.PASS) return { id, desc: "Pass / Confirm" };

        // Play Member (Simple) - convert to available_areas format
        if (id >= ActionBases.HAND && id < ActionBases.HAND_CHOICE) {
            const adj = id - ActionBases.HAND;
            const handIdx = Math.floor(adj / 3);
            const handCards = p.hand.cards;
            const card = typeof handCards[handIdx] === 'object' ? handCards[handIdx] : this.getCard(handCards[handIdx]);
            const cardId = card?.card_no || handCards[handIdx];
            const cardCost = card?.cost || 0;

            // Check which areas are available
            const stageCards = [p.stage.left_side, p.stage.center, p.stage.right_side];
            const energyCards = p.energy.cards;
            const activeEnergyCount = energyCards.filter(e => e && e.orientation !== 'Wait').length;

            const areaNames = ['left_side', 'center', 'right_side'];
            const availableAreas = [];

            for (let i = 0; i < 3; i++) {
                const existingCard = stageCards[i];
                let areaInfo = {
                    area: areaNames[i],
                    available: false,
                    cost: cardCost,
                    is_baton_touch: false,
                    existing_member_name: null
                };

                if (existingCard && existingCard.card_no !== -1) {
                    // Baton touch - check if enough energy
                    if (activeEnergyCount >= 1) {
                        const existingCost = existingCard.cost || 0;
                        const costToPay = Math.max(0, cardCost - existingCost);
                        if (activeEnergyCount >= costToPay) {
                            areaInfo.available = true;
                            areaInfo.cost = costToPay;
                            areaInfo.is_baton_touch = true;
                            areaInfo.existing_member_name = existingCard.name || `Card ${existingCard.card_no}`;
                        }
                    }
                } else {
                    // Play to empty area
                    if (activeEnergyCount >= cardCost) {
                        areaInfo.available = true;
                    }
                }

                availableAreas.push(areaInfo);
            }

            // Only return action if at least one area is available
            const hasAvailable = availableAreas.some(a => a.available);
            if (!hasAvailable) return null;

            return {
                id,
                index: id,
                type: 'PLAY',
                category: 'PLAY',
                action_type: 'PlayMemberToStage',
                hand_idx: handIdx,
                name: card ? card.name : "Unknown",
                img: card ? card._img : null,
                cost: cardCost,
                description: card ? `${card.name} (${card.card_no})` : "Unknown",
                parameters: {
                    card_id: cardId,
                    card_index: handIdx,
                    available_areas: availableAreas
                }
            };
        }

        // Play with Choice - convert to available_areas format
        if (id >= ActionBases.HAND_CHOICE && id < ActionBases.HAND_SELECT) {
            const adj = id - ActionBases.HAND_CHOICE;
            const handIdx = Math.floor(adj / 30);
            const handCards = p.hand.cards;
            const card = typeof handCards[handIdx] === 'object' ? handCards[handIdx] : this.getCard(handCards[handIdx]);
            const cardId = card?.card_no || handCards[handIdx];
            const cardCost = card?.cost || 0;

            // Check which areas are available
            const stageCards = [p.stage.left_side, p.stage.center, p.stage.right_side];
            const energyCards = p.energy.cards;
            const activeEnergyCount = energyCards.filter(e => e && e.orientation !== 'Wait').length;

            const areaNames = ['left_side', 'center', 'right_side'];
            const availableAreas = [];

            for (let i = 0; i < 3; i++) {
                const existingCard = stageCards[i];
                let areaInfo = {
                    area: areaNames[i],
                    available: false,
                    cost: cardCost,
                    is_baton_touch: false,
                    existing_member_name: null
                };

                if (existingCard && existingCard.card_no !== -1) {
                    // Baton touch
                    if (activeEnergyCount >= 1) {
                        const existingCost = existingCard.cost || 0;
                        const costToPay = Math.max(0, cardCost - existingCost);
                        if (activeEnergyCount >= costToPay) {
                            areaInfo.available = true;
                            areaInfo.cost = costToPay;
                            areaInfo.is_baton_touch = true;
                            areaInfo.existing_member_name = existingCard.name || `Card ${existingCard.card_no}`;
                        }
                    }
                } else {
                    if (activeEnergyCount >= cardCost) {
                        areaInfo.available = true;
                    }
                }

                availableAreas.push(areaInfo);
            }

            const hasAvailable = availableAreas.some(a => a.available);
            if (!hasAvailable) return null;

            return {
                id,
                index: id,
                type: 'PLAY',
                category: 'PLAY',
                action_type: 'PlayMemberToStage',
                hand_idx: handIdx,
                name: card ? card.name : "Unknown",
                img: card ? card._img : null,
                cost: cardCost,
                description: card ? `${card.name} (${card.card_no})` : "Unknown",
                parameters: {
                    card_id: cardId,
                    card_index: handIdx,
                    available_areas: availableAreas
                }
            };
        }

        // Select Hand / Discard
        if (id >= ActionBases.HAND_SELECT && id < ActionBases.STAGE) {
            const handIdx = id - ActionBases.HAND_SELECT;
            const handCards = p.hand.cards;
            const cardId = handCards[handIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'SELECT_HAND',
                hand_idx: handIdx,
                name: card ? card.name : "Unknown",
                img: card ? card._img : null,
                desc: `Select ${card ? card.name : 'Card'}`
            };
        }

        // Stage Ability (Simple & Choice)
        if (id >= ActionBases.STAGE && id < ActionBases.DISCARD_ACTIVATE) {
            // This range covers both STAGE and STAGE_CHOICE in the engine's current logic
            // Fix: Handle STAGE_CHOICE range separately since the offset is different
            let adj, slotIdx;
            if (id >= ActionBases.STAGE_CHOICE) {
                adj = id - ActionBases.STAGE_CHOICE;
                slotIdx = Math.floor(adj / 100);
            } else {
                adj = id - ActionBases.STAGE;
                slotIdx = Math.floor(adj / 100);
            }
            const abIdx = Math.floor((adj % 100) / 10);
            const stageCards = [p.stage.left_side, p.stage.center, p.stage.right_side];
            const cardId = stageCards[slotIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'ABILITY',
                area_idx: slotIdx,
                name: card ? card.name : "Unknown",
                img: card ? card._img : null,
                desc: id >= ActionBases.STAGE_CHOICE ? `Activate ${card ? card.name : 'Card'} (with choice)` : `Activate ${card ? card.name : 'Card'}`,
            };
        }

        // Activate from Discard
        if (id >= ActionBases.DISCARD_ACTIVATE && id < ActionBases.CHOICE) {
            const adj = id - ActionBases.DISCARD_ACTIVATE;
            const discardIdx = Math.floor(adj / 10);
            const abIdx = adj % 10;
            const discardCards = p.waitroom.cards;
            const cardId = discardCards[discardIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'SELECT_DISCARD',
                discard_idx: discardIdx,
                ab_idx: abIdx,
                name: card ? card.name : "Unknown",
                img: card ? card._img : null,
                desc: `Activate ${card ? card.name : 'Card'} from Discard`
            };
        }

        // Mode Select
        if (id >= ActionBases.MODE && id < ActionBases.LIVESET) {
            const index = id - ActionBases.MODE;
            return { id, type: 'SELECT_MODE', index, desc: `Select Mode ${index}` };
        }

        // Live Set
        if (id >= ActionBases.LIVESET && id < ActionBases.COLOR) {
            const handIdx = id - ActionBases.LIVESET;
            return { id, type: 'PLACE_LIVE', hand_idx: handIdx, desc: `Set Live Card ${handIdx}` };
        }

        // Color Selection
        if (id >= ActionBases.COLOR && id < ActionBases.COLOR + 7) {
            const colorIdx = id - ActionBases.COLOR;
            const colors = ["Pink", "Red", "Yellow", "Green", "Blue", "Purple", "All"];
            return { id, type: 'SELECT_COLOR', index: colorIdx, desc: `Select Color: ${colors[colorIdx] || colorIdx}` };
        }

        // Stage Slot Selection
        if (id >= ActionBases.STAGE_SLOTS && id < ActionBases.STAGE_SLOTS + 3) {
            const slotIdx = id - ActionBases.STAGE_SLOTS;
            return { id, type: 'SELECT_SLOT', index: slotIdx, desc: `Select Slot ${slotIdx}` };
        }

        // Generic Interaction Choice (LOOK_AND_CHOOSE, etc.)
        if (id >= ActionBases.CHOICE) {
            return { id, type: 'SELECT', index: id - ActionBases.CHOICE, desc: `Choice ${id - ActionBases.CHOICE}` };
        }

        return { id, desc: `Action ${id}` };
    }

    getCard(id) {
        // Use State.resolveCardData which handles both game state cards and static database
        return State.resolveCardData(id);
    }
}

// Singleton instance
export const wasmAdapter = new WasmAdapter();
