import { State } from '../state.js';
import { Network } from '../network.js';
import { Modals } from '../ui_modals.js';
import { validator } from '../components/DeckValidator.js';
import { ModalManager } from '../utils/ModalManager.js';

let deckModalInputBound = false;

export const DeckSetupModal = {
    openDeckModal: () => {
        ModalManager.show('deck-modal');
        validator.init();
        Modals.fetchAndPopulateDecks();

        const input = document.getElementById('deck-html-input');
        if (input && !deckModalInputBound) {
            input.addEventListener('input', () => DeckSetupModal.validateInline());
            deckModalInputBound = true;
        }
    },

    validateInline: () => {
        const input = document.getElementById('deck-html-input');
        const preview = document.getElementById('deck-preview');
        if (!input || !preview) return;

        const results = validator.validateDeckString(input.value);
        validator.renderPreview(results, preview);
    },

    closeDeckModal: () => {
        ModalManager.hide('deck-modal');
    },

    fetchAndPopulateDecks: async () => {
        try {
            const resp = await fetch('api/get_decks');
            const data = await resp.json();
            if (data.success && data.decks) {
                Modals.deckPresets = data.decks;
                DeckSetupModal.populateDeckSelect('deck-preset-select', data.decks);
            } else {
                Modals.deckPresets = [];
                console.error("Fetch decks success but data missing or wrong key", data);
            }
        } catch (e) {
            Modals.deckPresets = [];
            console.error("Failed to fetch decks", e);
        }
    },

    populateDeckSelect: (elementId, decks) => {
        const select = document.getElementById(elementId);
        if (!select) return;

        const manual = select.querySelector('option[value="manual"]');
        const paste = select.querySelector('option[value="paste"]');
        const random = select.querySelector('option[value="random"]');

        select.innerHTML = '';
        if (manual) select.appendChild(manual);
        if (paste) select.appendChild(paste);

        (decks || []).forEach(d => {
            const opt = document.createElement('option');
            opt.value = d.id;
            opt.textContent = `${d.name} (${d.card_count} cards)`;
            select.appendChild(opt);
        });

        if (random) select.appendChild(random);

        if (decks && decks.length > 0) {
            select.value = decks[0].id;
        } else if (paste) {
            select.value = 'paste';
        } else if (manual) {
            select.value = 'manual';
        }
    },

    submitDeck: async () => {
        const playerVal = document.getElementById('deck-player-select').value;
        let content = '';

        const fileInput = document.getElementById('deck-file-input');
        const textInput = document.getElementById('deck-html-input').value;

        if (fileInput && fileInput.files[0]) {
            try {
                content = await fileInput.files[0].text();
            } catch (e) {
                alert("Failed to read file: " + e.message);
                return;
            }
        } else if (textInput.trim()) {
            content = textInput;
        } else {
            alert('Please select a file or paste deck HTML.');
            return;
        }

        const results = validator.validateDeckString(content);
        if (results.parsed.length === 0 && results.parsedEnergy.length === 0) {
            alert("No cards found in the provided content.");
            return;
        }

        let mainDeck = [];
        let energyDeck = [];

        results.parsed.forEach(p => {
            for (let i = 0; i < p.count; i++) mainDeck.push(p.code);
        });
        results.parsedEnergy.forEach(p => {
            for (let i = 0; i < p.count; i++) energyDeck.push(p.code);
        });

        const playerIds = (playerVal === 'both') ? [0, 1] : [parseInt(playerVal)];

        for (const pid of playerIds) {
            try {
                const resp = await fetch('api/set_deck', {
                    method: 'POST',
                    headers: Network.getHeaders(),
                    body: JSON.stringify({
                        player: pid,
                        deck: mainDeck,
                        energy_deck: energyDeck
                    })
                });
                const result = await resp.json();

                if (result.success) {
                    console.log(`Deck set for Player ${pid + 1}`);
                } else {
                    alert(`Failed for P${pid + 1}: ` + (result.error || 'Unknown error'));
                }
            } catch (e) {
                alert(`Error for P${pid + 1}: ` + e.message);
            }
        }
        DeckSetupModal.closeDeckModal();
        if (State.roomCode || State.offlineMode || State.gameHasStarted) {
            await Network.fetchState();
        }
    },

    loadTestDeck: async () => {
        const playerVal = document.getElementById('deck-player-select').value;
        const playerIds = (playerVal === 'both') ? [0, 1] : [parseInt(playerVal)];

        if (!confirm(`Load 'Test Deck' for Player ${playerVal === 'both' ? 'Both' : parseInt(playerVal) + 1}?`)) return;

        try {
            const res = await fetch('api/get_test_deck');
            const data = await res.json();
            if (!data.success) {
                alert("Failed to load test deck: " + data.error);
                return;
            }

            const cards = data.content;
            for (const pid of playerIds) {
                const resp = await fetch('api/set_deck', {
                    method: 'POST',
                    headers: Network.getHeaders(),
                    body: JSON.stringify({
                        player: pid,
                        deck: cards,
                        energy_deck: []
                    })
                });
                const result = await resp.json();
                if (result.success) {
                    console.log(`Test Deck loaded for P${pid + 1}`);
                }
            }
            DeckSetupModal.closeDeckModal();
            if (State.roomCode || State.offlineMode || State.gameHasStarted) {
                await Network.fetchState();
            }
        } catch (e) {
            console.error(e);
            alert("Error loading test deck: " + e.message);
        }
    },

    loadRandomDeck: async () => {
        const playerVal = document.getElementById('deck-player-select').value;
        const playerIds = (playerVal === 'both') ? [0, 1] : [parseInt(playerVal)];

        if (!confirm(`Generate Random Deck for Player ${playerVal === 'both' ? 'Both' : parseInt(playerVal) + 1}?`)) return;

        try {
            const res = await fetch('api/get_random_deck');
            const data = await res.json();
            if (!data.success) {
                alert("Failed to generate deck: " + data.error);
                return;
            }

            const cards = data.content;
            for (const pid of playerIds) {
                const resp = await fetch('api/set_deck', {
                    method: 'POST',
                    headers: Network.getHeaders(),
                    body: JSON.stringify({
                        player: pid,
                        deck: cards,
                        energy_deck: []
                    })
                });
                const result = await resp.json();
                if (result.success) {
                    console.log(`Random Deck Loaded for P${pid + 1}`);
                }
            }
            DeckSetupModal.closeDeckModal();
            if (State.roomCode || State.offlineMode || State.gameHasStarted) {
                await Network.fetchState();
            }
        } catch (e) {
            console.error(e);
            alert("Error loading random deck: " + e.message);
        }
    }
};
