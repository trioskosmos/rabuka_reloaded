import { State } from '../state.js';
import { Network } from '../network.js';
import { Modals } from '../ui_modals.js';
import { validator } from '../components/DeckValidator.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS, DISPLAY_VALUES } from '../constants_dom.js';

const inlineValidationBound = new Set();

function bindInlineValidation(playerId) {
    if (inlineValidationBound.has(playerId)) {
        return;
    }

    const input = document.getElementById(`p${playerId}-deck-paste`);
    if (!input) {
        return;
    }

    input.addEventListener('input', () => GameSetupModal.validateInline(playerId));
    inlineValidationBound.add(playerId);
}

export const GameSetupModal = {
    openSetupModal: (mode) => {
        Modals.setupMode = mode;
        ModalManager.show(DOM_IDS.MODAL_SETUP);
        ModalManager.hide(DOM_IDS.MODAL_ROOM);

        validator.init();

        Modals.fetchAndPopulateDecks().then(() => {
            Modals.populateDeckSelect('p0-deck-select', Modals.deckPresets);
            Modals.populateDeckSelect('p1-deck-select', Modals.deckPresets);
        });

        bindInlineValidation(0);
        bindInlineValidation(1);

        const p0Col = document.getElementById('setup-p0-col');
        const p1Col = document.getElementById('setup-p1-col');
        const title = document.getElementById('setup-title');
        if (title) title.textContent = (mode === 'pvp') ? 'PvP Setup' : 'PvE Setup';

        if (p0Col) p0Col.style.display = DISPLAY_VALUES.BLOCK;
        if (p1Col) {
            if (mode === 'pvp') {
                p1Col.style.display = DISPLAY_VALUES.NONE;
            } else {
                p1Col.style.display = DISPLAY_VALUES.BLOCK;
                p1Col.style.opacity = '1';
                p1Col.style.pointerEvents = 'auto';
                const p1Title = p1Col.querySelector('h4');
                if (p1Title) p1Title.textContent = (mode === 'pve') ? '[AI] Player 2 (AI)' : '[P2] Player 2 (Opponent)';
            }
        }
    },

    validateInline: (pid) => {
        const input = document.getElementById(`p${pid}-deck-paste`);
        const preview = document.getElementById(`p${pid}-deck-preview`);
        if (!input || !preview) return;

        const results = validator.validateDeckString(input.value);
        validator.renderPreview(results, preview);
    },

    closeSetupModal: () => {
        ModalManager.hide(DOM_IDS.MODAL_SETUP);
    },

    getDeckConfig: (pid) => {
        const selectId = `p${pid}-deck-select`;
        const select = document.getElementById(selectId);
        if (!select) return null;

        const mode = select.value;
        if (mode === 'manual' || mode === 'paste') {
            let input = document.getElementById(`p${pid}-manual-deck`);
            if (!input) input = document.getElementById(`p${pid}-deck-paste`);
            return { type: 'manual', content: input ? input.value : '' };
        } else if (mode === 'random') {
            return { type: 'random' };
        } else {
            const presets = Modals.deckPresets || [];
            const preset = presets.find(d => d.id === mode);
            return { type: 'preset', id: mode, preset: preset };
        }
    },

    resolveDeck: async (config) => {
        if (!config) return null;
        if (config.type === 'preset') {
            if (!config.preset) {
                config.preset = Modals.deckPresets.find(d => d.id === config.id);
            }
            if (!config.preset) {
                console.error("Preset not found:", config.id);
                return null;
            }
            return { main: config.preset.main, energy: config.preset.energy };
        } else if (config.type === 'random') {
            const res = await fetch('api/get_random_deck');
            const data = await res.json();
            return {
                main: data.content || [],
                energy: data.energy || []
            };
        } else if (config.type === 'manual') {
            const content = config.content || "";
            const validation = validator.validateDeckString(content);

            const main = [];
            for (const item of validation.parsed) {
                if (item.valid) {
                    for (let i = 0; i < item.count; i++) {
                        main.push(item.code);
                    }
                }
            }

            const energy = [];
            for (const item of validation.parsedEnergy) {
                if (item.valid) {
                    for (let i = 0; i < item.count; i++) {
                        energy.push(item.code);
                    }
                }
            }

            return { main, energy };
        }
        return null;
    },

    submitGameSetup: async () => {
        const p0Config = GameSetupModal.getDeckConfig(0);
        const p1Config = GameSetupModal.getDeckConfig(1);
        const cardSetSelect = document.getElementById('card-set-select');
        const cardSet = cardSetSelect ? cardSetSelect.value : 'compiled';

        try {
            const p0Deck = await GameSetupModal.resolveDeck(p0Config);
            const p1Deck = await GameSetupModal.resolveDeck(p1Config);

            if (!p0Deck || !p1Deck) {
                alert("Failed to resolve decks. Please check console.");
                return;
            }

            const payload = {
                mode: Modals.setupMode,
                card_set: cardSet,
                p0_deck: p0Deck.main,
                p0_energy: p0Deck.energy,
                public: true
            };

            // Only send P1 deck if NOT in manual PVP mode (Host sets it for PvE or Hotseat)
            // Hotseat is currently handled via 'pve' mode with toggle-hotseat,
            // but if we ever use 'pvp' for hotseat this might need adjustment.
            if (Modals.setupMode !== 'pvp') {
                payload.p1_deck = p1Deck.main;
                payload.p1_energy = p1Deck.energy;
            }

            const res = await fetch('api/rooms/create', {
                method: 'POST',
                headers: Network.getHeaders(),
                body: JSON.stringify(payload)
            });

            if (!res.ok) {
                const errorData = await res.json().catch(() => ({ error: "Server error" }));
                throw new Error(errorData.error || `HTTP error! status: ${res.status}`);
            }

            const data = await res.json();
            if (data.success) {
                State.roomCode = data.room_id;
                State.cardSet = cardSet;
                State.offlineMode = false;
                if (data.session) {
                    Network.saveSession(data.room_id, data.session);
                }
                localStorage.setItem('lovelive_room_code', State.roomCode);

                const waitingHint = document.getElementById('room-waiting-hint');
                if (waitingHint && Modals.setupMode === 'pvp') {
                    waitingHint.textContent = `Waiting for an opponent. Room code: ${State.roomCode}`;
                }

                if (Modals.setupMode === 'pvp') {
                    ModalManager.show(DOM_IDS.MODAL_ROOM);
                } else {
                    ModalManager.hide(DOM_IDS.MODAL_ROOM);
                }

                GameSetupModal.closeSetupModal();

                window.onRoomUpdate?.();
                await Network.fetchState();
            } else {
                alert("Failed to create game: " + data.error);
            }
        } catch (e) {
            console.error(e);
            alert("Network error: " + e.message);
        }
    },

    startGame: async (mode = 'pve') => {
        // Simplified: Skip room system, initialize game directly
        try {
            const res = await fetch('api/init', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' }
            });
            const data = await res.json();

            if (!res.ok) {
                throw new Error(data?.error || `Failed to initialize game (${res.status})`);
            }

            if (data) {
                State.offlineMode = false;
                State.roomCode = null;
                State.gameHasStarted = true;
                localStorage.removeItem('lovelive_room_code');

                ModalManager.hide(DOM_IDS.MODAL_ROOM);
                console.log(`[GameSetup] Game started (${mode})`);

                // Fetch initial state
                if (Network.fetchState) {
                    await Network.fetchState();
                }
            } else {
                alert('Failed to start game');
            }
        } catch (e) {
            console.error(e);
            alert('Network error starting game');
        }
    },

    openDeckSelectionForPvP: (pid) => {
        Modals.pvpJoinPid = pid;
        ModalManager.show(DOM_IDS.MODAL_SETUP);

        const p0Col = document.getElementById('setup-p0-col');
        const p1Col = document.getElementById('setup-p1-col');
        const startBtn = document.getElementById('setup-start-btn');
        const title = document.getElementById('setup-title');

        if (title) title.textContent = 'Select Your Deck';

        if (pid === 0) {
            if (p0Col) p0Col.style.display = DISPLAY_VALUES.BLOCK;
            if (p1Col) p1Col.style.display = DISPLAY_VALUES.NONE;
        } else {
            if (p0Col) p0Col.style.display = DISPLAY_VALUES.NONE;
            if (p1Col) {
                p1Col.style.display = DISPLAY_VALUES.BLOCK;
                p1Col.style.opacity = '1';
                p1Col.style.pointerEvents = 'auto';
            }
        }

        if (startBtn) {
            startBtn.textContent = 'Submit Deck & Join';
            startBtn.onclick = GameSetupModal.submitPvPDeck;
        }

        Modals.fetchAndPopulateDecks().then(() => {
            const selectId = pid === 0 ? 'p0-deck-select' : 'p1-deck-select';
            Modals.populateDeckSelect(selectId, Modals.deckPresets);
        });
    },

    submitPvPDeck: async () => {
        const config = GameSetupModal.getDeckConfig(Modals.pvpJoinPid);
        const resolved = await GameSetupModal.resolveDeck(config);

        if (!resolved) return;

        try {
            const res = await fetch('api/set_deck', {
                method: 'POST',
                headers: Network.getHeaders(),
                body: JSON.stringify({
                    player: Modals.pvpJoinPid,
                    deck: resolved.main,
                    energy_deck: resolved.energy
                })
            });
            const data = await res.json();
            if (data.success || data.status === 'ok') {
                GameSetupModal.closeSetupModal();
                await Network.fetchState();
                alert("Deck Submitted! Waiting for game to start.");
            } else {
                alert("Error setting deck: " + (data.error || "Unknown"));
            }
        } catch (e) {
            console.error(e);
            alert("Error submitting deck.");
        }
    },

    onDeckSelectChange: (pid, value) => {
        let finalValue = value;
        if (finalValue === undefined) {
            const select = document.getElementById(`p${pid}-deck-select`);
            if (select) finalValue = select.value;
        }
        console.log(`Player ${pid} selected deck: ${finalValue}`);
        const pasteArea = document.getElementById(`p${pid}-paste-area`);
        if (pasteArea) {
            pasteArea.style.display = (finalValue === 'paste' || finalValue === 'manual') ? DISPLAY_VALUES.BLOCK : DISPLAY_VALUES.NONE;
        }
    }
};
