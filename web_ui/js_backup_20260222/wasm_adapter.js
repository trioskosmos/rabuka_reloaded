
import init, { WasmEngine } from '../pkg/engine_rust.js';
import { ActionBases } from './generated_constants.js';
import { wasmLoader } from './wasm_loader.js';

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

                // Load Card DB
                const base = getAppBaseUrl();
                const res = await fetch(`${base}data/cards_compiled.json`);
                const text = await res.text();
                this.cardDbRaw = text;
                this.cardDb = JSON.parse(text); // Keep a JS copy for lookups

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
        if (!this.initialized) await this.init();
        if (!this.cardDb) throw new Error("Card database not loaded");
        if (!this.cardMap) this.buildCardMap();

        const deck = [];
        const energy = [];
        const lives = [];

        if (!deckList || !Array.isArray(deckList)) return { deck, energy, lives };

        deckList.forEach(rawId => {
            let info = null;
            if (typeof rawId === 'number') {
                info = this.cardDb.member_db[rawId] || this.cardDb.energy_db[rawId];
                if (!info && this.cardDb.live_db) info = this.cardDb.live_db[rawId];
            } else {
                info = this.cardMap[rawId];
            }

            if (info) {
                const id = info.card_id;
                if (this.cardDb.energy_db[id]) energy.push(id);
                else if (this.cardDb.live_db && this.cardDb.live_db[id]) lives.push(id);
                else deck.push(id);
            }
        });

        return { deck, energy, lives };
    }

    buildCardMap() {
        if (!this.cardDb) return;
        this.cardMap = {};
        const dbs = [this.cardDb.member_db, this.cardDb.live_db, this.cardDb.energy_db];
        for (const db of dbs) {
            if (!db) continue;
            for (const key in db) {
                const card = db[key];
                if (card.card_no) this.cardMap[card.card_no] = card;
                this.cardMap[card.card_id] = card; // Also map ID
            }
        }
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
        return Array.from(rawIds).map(id => this.enrichAction(id, state));
    }

    enrichAction(id, state) {
        // Logic to reverse-engineer action details from ID and State
        const p = state.players[state.current_player];

        if (id === ActionBases.PASS) return { id, desc: "Pass / Confirm" };

        // Play Member (Simple)
        if (id >= ActionBases.HAND && id < ActionBases.HAND_CHOICE) {
            const adj = id - ActionBases.HAND;
            const handIdx = Math.floor(adj / 3);
            const slotIdx = adj % 3;
            const cardId = p.hand[handIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'PLAY',
                hand_idx: handIdx,
                area_idx: slotIdx,
                name: card ? card.name : "Unknown",
                img: card ? (card.img_path.startsWith('img/') ? card.img_path : 'img/' + card.img_path) : null,
                cost: card ? card.cost : 0,
                desc: `Play ${card ? card.name : 'Card'} to Slot ${slotIdx}`
            };
        }

        // Play with Choice
        if (id >= ActionBases.HAND_CHOICE && id < ActionBases.HAND_SELECT) {
            const adj = id - ActionBases.HAND_CHOICE;
            const handIdx = Math.floor(adj / 30);
            const slotIdx = Math.floor((adj % 30) / 10);
            const cardId = p.hand[handIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'PLAY',
                hand_idx: handIdx,
                area_idx: slotIdx,
                name: card ? card.name : "Unknown",
                img: card ? (card.img_path.startsWith('img/') ? card.img_path : 'img/' + card.img_path) : null,
                desc: `Play ${card ? card.name : 'Card'} to Slot ${slotIdx} (with choice)`
            };
        }

        // Select Hand / Discard
        if (id >= ActionBases.HAND_SELECT && id < ActionBases.STAGE) {
            const handIdx = id - ActionBases.HAND_SELECT;
            const cardId = p.hand[handIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'SELECT_HAND',
                hand_idx: handIdx,
                name: card ? card.name : "Unknown",
                img: card ? (card.img_path.startsWith('img/') ? card.img_path : 'img/' + card.img_path) : null,
                desc: `Select ${card ? card.name : 'Card'}`
            };
        }

        // Stage Ability (Simple & Choice)
        if (id >= ActionBases.STAGE && id < ActionBases.DISCARD_ACTIVATE) {
            // This range covers both STAGE and STAGE_CHOICE in the engine's current logic
            const adj = id - ActionBases.STAGE;
            const slotIdx = Math.floor(adj / 100);
            const abIdx = Math.floor((adj % 100) / 10);
            const cardId = p.stage[slotIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'ABILITY',
                area_idx: slotIdx,
                name: card ? card.name : "Unknown",
                img: card ? (card.img_path.startsWith('img/') ? card.img_path : 'img/' + card.img_path) : null,
                desc: id >= ActionBases.STAGE_CHOICE ? `Activate ${card ? card.name : 'Card'} (with choice)` : `Activate ${card ? card.name : 'Card'}`,
            };
        }

        // Activate from Discard
        if (id >= ActionBases.DISCARD_ACTIVATE && id < ActionBases.CHOICE) {
            const adj = id - ActionBases.DISCARD_ACTIVATE;
            const discardIdx = Math.floor(adj / 10);
            const abIdx = adj % 10;
            const cardId = p.discard[discardIdx];
            const card = this.getCard(cardId);
            return {
                id,
                type: 'SELECT_DISCARD',
                discard_idx: discardIdx,
                ab_idx: abIdx,
                name: card ? card.name : "Unknown",
                img: card ? (card.img_path.startsWith('img/') ? card.img_path : 'img/' + card.img_path) : null,
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

        // Fallback
        return { id, desc: `Action ${id}` };
    }

    getCard(id) {
        if (!this.cardDb) return null;
        return this.cardDb.member_db[id] || (this.cardDb.live_db ? this.cardDb.live_db[id] : null) || this.cardDb.energy_db[id];
    }
}

// Singleton instance
export const wasmAdapter = new WasmAdapter();
