import { State } from '../state.js';
import { Network } from '../network.js';
import { Modals } from '../ui_modals.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS, DISPLAY_VALUES } from '../constants_dom.js';

export const GameSetupModal = {
    openSetupModal: (mode) => {
        Modals.setupMode = mode;
        ModalManager.show(DOM_IDS.MODAL_SETUP);
        ModalManager.hide(DOM_IDS.MODAL_ROOM);

        Modals.fetchAndPopulateDecks().then(() => {
            Modals.populateDeckSelect('p0-deck-select', Modals.deckPresets);
            Modals.populateDeckSelect('p1-deck-select', Modals.deckPresets);
        });

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
            // Parse deck content from paste textarea
            const content = config.content || '';
            if (!content.trim()) {
                console.warn("Manual deck is empty");
                return { main: [], energy: [] };
            }
            // Parse: each line can be "ID x Count" or just "ID"
            const lines = content.split(/\r?\n/);
            const main = [];
            const energy = [];
            for (let line of lines) {
                line = line.trim();
                if (!line || line.startsWith('#')) continue;
                let cardNo = '';
                let qty = 1;
                const xMatch = line.match(/^(.+?)\s*[xX×]\s*(\d+)$/);
                if (xMatch) {
                    cardNo = xMatch[1].trim().toUpperCase();
                    qty = parseInt(xMatch[2], 10);
                } else {
                    cardNo = line.toUpperCase();
                    qty = 1;
                }
                if (!cardNo || !cardNo.includes('-')) continue;
                // Energy cards typically have "PE" or "E" in their code
                if (cardNo.includes('-PE') || cardNo.includes('-E')) {
                    for (let i = 0; i < qty; i++) energy.push(cardNo);
                } else {
                    for (let i = 0; i < qty; i++) main.push(cardNo);
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

            // Store custom decks via set_deck, then init the game
            await fetch('api/set_deck', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ player: 0, deck: p0Deck.main })
            });
            if (Modals.setupMode !== 'pvp') {
                await fetch('api/set_deck', {
                    method: 'POST', headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ player: 1, deck: p1Deck.main })
                });
            }

            const initRes = await fetch('api/init', {
                method: 'POST', headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({})
            });

            if (!initRes.ok) {
                const errorData = await initRes.json().catch(() => ({ error: "Server error" }));
                throw new Error(errorData.error || `HTTP error! status: ${initRes.status}`);
            }

            const data = await initRes.json();
            State.offlineMode = false;

            if (Modals.setupMode === 'pvp') {
                ModalManager.show(DOM_IDS.MODAL_ROOM);
            } else {
                ModalManager.hide(DOM_IDS.MODAL_ROOM);
            }
            GameSetupModal.closeSetupModal();
            await Network.fetchState();
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
