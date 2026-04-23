/**
 * UI Modals Module
 * Handles modal opening, closing, and internal logic for Deck, Settings, Setup, etc.
 */
import { State } from './state.js';
import { Network } from './network.js';
import { Rendering } from './ui_rendering.js';
// Let's use the local names if needed
// import { Tooltips } from './ui_tooltips.js';

export const Modals = {
    deckPresets: [],
    setupMode: 'pve',
    pvpJoinPid: 1,

    // --- Deck Modal ---
    openDeckModal: () => {
        const modal = document.getElementById('deck-modal');
        if (modal) modal.style.display = 'flex';
        Modals.fetchAndPopulateDecks();
    },

    closeDeckModal: () => {
        const modal = document.getElementById('deck-modal');
        if (modal) modal.style.display = 'none';
    },

    fetchAndPopulateDecks: async () => {
        try {
            const resp = await fetch('api/get_decks');
            const data = await resp.json();
            if (data.success && data.decks) {
                Modals.deckPresets = data.decks;
                Modals.populateDeckSelect('deck-preset-select', data.decks);
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

        // Preserve "Manual" and "Random" options if they exist
        const manual = select.querySelector('option[value="manual"]');
        const random = select.querySelector('option[value="random"]');

        select.innerHTML = '';
        if (manual) select.appendChild(manual);

        (decks || []).forEach(d => {
            const opt = document.createElement('option');
            opt.value = d.id;
            opt.textContent = `${d.name} (${d.card_count} cards)`;
            select.appendChild(opt);
        });

        // "Random" option at the bottom
        if (random) select.appendChild(random);

        // Default to the first preset instead of "Random" (if presets exist)
        if (decks && decks.length > 0) {
            select.value = decks[0].id;
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

        // Client-side parsing to match backend logic
        // Matches "CardCode x N" (Suffix), "N x CardCode" (Prefix), or just "CardCode"
        // Also supports LL- prefixes and special chars
        const lines = content.split(/\r?\n|,\s*/);
        let mainDeck = [];
        let energyDeck = []; // We generally don't parse energy from text imports for now, or assume all are main

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed.startsWith('#')) continue;

            let count = 1;
            let code = trimmed;

            // Try "ID x N"
            const suffixMatch = trimmed.match(/^(.+?)\s*[xX]\s*(\d+)$/);
            if (suffixMatch) {
                code = suffixMatch[1].trim();
                count = parseInt(suffixMatch[2]);
            } else {
                // Try "N x ID"
                const prefixMatch = trimmed.match(/^(\d+)\s*[xX]\s*(.+)$/);
                if (prefixMatch) {
                    count = parseInt(prefixMatch[1]);
                    code = prefixMatch[2].trim();
                }
            }

            // Clean up code (remove quotes if any, though usually not needed for raw text)
            // Just basic cleanup

            for (let i = 0; i < count; i++) mainDeck.push(code);
        }

        const playerIds = (playerVal === 'both') ? [0, 1] : [parseInt(playerVal)];

        for (const pid of playerIds) {
            try {
                // Use set_deck instead of upload_deck
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
        Modals.closeDeckModal();
        if (window.fetchState) window.fetchState();
    },

    // --- Game Setup Modal ---
    openSetupModal: (mode) => {
        Modals.setupMode = mode;
        const modal = document.getElementById('setup-modal');
        if (modal) modal.style.display = 'flex';

        // Hide lobby modal if it's open
        const roomModal = document.getElementById('room-modal');
        if (roomModal) roomModal.style.display = 'none';

        // Populate decks immediately
        Modals.fetchAndPopulateDecks().then(() => {
            // Refresh specific selectors if needed, but fetchAndPopulateDecks handles 'deck-preset-select'
            // We also need to populate p0 and p1 selectors for detailed setup
            Modals.populateDeckSelect('p0-deck-select', Modals.deckPresets);
            Modals.populateDeckSelect('p1-deck-select', Modals.deckPresets);
        });

        const p0Col = document.getElementById('setup-p0-col');
        const p1Col = document.getElementById('setup-p1-col');
        const title = document.getElementById('setup-title');
        if (title) title.textContent = (mode === 'pvp') ? 'PvP Setup' : 'PvE Setup';

        // Both columns should be visible to select decks
        if (p0Col) p0Col.style.display = 'block';
        if (p1Col) {
            p1Col.style.display = 'block';
            p1Col.style.opacity = '1';
            p1Col.style.pointerEvents = 'auto';
            // Update AI/Player label
            const p1Title = p1Col.querySelector('h4');
            if (p1Title) p1Title.textContent = (mode === 'pve') ? '🤖 Player 2 (AI)' : '👤 Player 2 (Opponent)';
        }
        Modals.fetchAndPopulateDecks();
    },

    closeSetupModal: () => {
        const modal = document.getElementById('setup-modal');
        if (modal) modal.style.display = 'none';

        // Restore lobby if no room code
        if (!State.roomCode && !State.offlineMode && !State.replayMode) {
            const roomModal = document.getElementById('room-modal');
            if (roomModal) roomModal.style.display = 'flex';
        }
    },

    getDeckConfig: (pid) => {
        const selectId = `p${pid}-deck-select`;
        const select = document.getElementById(selectId);
        if (!select) return null;

        const mode = select.value;
        if (mode === 'manual' || mode === 'paste') {
            // Check for both ID patterns
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
                // Try to find it again if it was somehow lost
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
            // Robust check for manual/paste
            const content = config.content || "";
            // If it looks like HTML from decklog, try regex first
            let matches = content.match(/(PL![A-Za-z0-9\-]+)/g);
            if (!matches || matches.length === 0) {
                // Fallback to line-by-line / raw ID parsing
                const lines = content.split(/\r?\n|,\s*/);
                matches = [];
                for (const line of lines) {
                    const trimmed = line.trim();
                    if (!trimmed || trimmed.startsWith('#')) continue;
                    // Simple heuristic: if it looks like a card ID (alphanumeric, maybe dashes)
                    // cleanup "x N" if present
                    let code = trimmed.replace(/^(\d+)[xX]\s*/, '').replace(/\s*[xX](\d+)$/, '').trim();
                    if (code.length > 0) matches.push(code);
                }
            }
            return { main: matches || [], energy: [] };
        }
        return null;
    },

    submitGameSetup: async () => {
        const p0Config = Modals.getDeckConfig(0);
        // "Take out AI default" - Allow manual selection for P2 even in PvE
        const p1Config = Modals.getDeckConfig(1);

        try {
            const p0Deck = await Modals.resolveDeck(p0Config);
            const p1Deck = await Modals.resolveDeck(p1Config);

            if (!p0Deck || !p1Deck) {
                alert("Failed to resolve decks. Please check console.");
                return;
            }

            const res = await fetch('api/rooms/create', {
                method: 'POST',
                headers: Network.getHeaders(),
                body: JSON.stringify({
                    mode: Modals.setupMode,
                    p0_deck: p0Deck.main,
                    p1_deck: p1Deck.main,
                    p0_energy: p0Deck.energy,
                    p1_energy: p1Deck.energy,
                    public: true
                })
            });

            if (!res.ok) {
                const errorData = await res.json().catch(() => ({ error: "Server error" }));
                throw new Error(errorData.error || `HTTP error! status: ${res.status}`);
            }

            const data = await res.json();
            if (data.success) {
                State.roomCode = data.room_id;
                State.offlineMode = false;
                // Persistent storage required for resume functionality
                if (data.session) {
                    Network.saveSession(data.room_id, { token: data.session, playerId: data.player_idx });
                }
                localStorage.setItem('lovelive_room_code', State.roomCode);

                // Hide lobbies
                const roomModal = document.getElementById('room-modal');
                if (roomModal) roomModal.style.display = 'none';

                // IMPORTANT: Hide the setup modal itself!
                Modals.closeSetupModal();

                // Update Header UI
                if (window.onRoomUpdate) window.onRoomUpdate();

                if (window.fetchState) window.fetchState();
            } else {
                alert("Failed to create game: " + data.error);
            }
        } catch (e) {
            console.error(e);
            alert("Network error: " + e.message);
        }
    },

    // --- PvP Join Modal ---
    openDeckSelectionForPvP: (pid) => {
        Modals.pvpJoinPid = pid;
        const modal = document.getElementById('setup-modal'); // Re-use setup modal
        if (modal) modal.style.display = 'flex';

        const p0Col = document.getElementById('setup-p0-col');
        const p1Col = document.getElementById('setup-p1-col');
        const startBtn = document.getElementById('setup-start-btn');
        const title = document.getElementById('setup-title');

        if (title) title.textContent = 'Select Your Deck';

        if (pid === 0) {
            if (p0Col) p0Col.style.display = 'block';
            if (p1Col) p1Col.style.display = 'none';
        } else {
            if (p0Col) p0Col.style.display = 'none';
            if (p1Col) {
                p1Col.style.display = 'block';
                p1Col.style.opacity = '1';
                p1Col.style.pointerEvents = 'auto';
            }
        }

        if (startBtn) {
            startBtn.textContent = '✅ Submit Deck & Join';
            startBtn.onclick = Modals.submitPvPDeck;
        }

        Modals.fetchAndPopulateDecks();
    },

    submitPvPDeck: async () => {
        const config = Modals.getDeckConfig(Modals.pvpJoinPid);
        const resolved = await Modals.resolveDeck(config);

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
                Modals.closeSetupModal();
                if (window.fetchState) window.fetchState();
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
        // Toggle paste area visibility
        const pasteArea = document.getElementById(`p${pid}-paste-area`);
        if (pasteArea) {
            pasteArea.style.display = (finalValue === 'paste' || finalValue === 'manual') ? 'block' : 'none';
        }
    },

    // --- Settings Modal ---
    openSettingsModal: () => {
        const modal = document.getElementById('settings-modal');
        if (modal) modal.style.display = 'flex';
    },

    closeSettingsModal: () => {
        const modal = document.getElementById('settings-modal');
        if (modal) modal.style.display = 'none';
    },

    updateBoardScale: (val) => {
        const scale = parseFloat(val);
        document.documentElement.style.setProperty('--zoom-override', scale);
        const zoomValue = document.getElementById('zoom-value');
        if (zoomValue) zoomValue.textContent = scale.toFixed(2);
        localStorage.setItem('lovelive_board_scale', scale);

        // Update slider position if it's not the one being touched
        const slider = document.getElementById('zoom-slider');
        if (slider && slider.value != val) slider.value = val;
    },

    toggleLang: () => {
        State.currentLang = State.currentLang === 'jp' ? 'en' : 'jp';
        Modals.updateLanguage();
    },

    toggleFriendlyAbilities: () => {
        State.showFriendlyAbilities = !State.showFriendlyAbilities;
        localStorage.setItem('lovelive_friendly_abilities', State.showFriendlyAbilities);
        Modals.updateLanguage();
        if (window.render) window.render();
    },

    updateLanguage: () => {
        if (!window.translations) return;
        const t = window.translations[State.currentLang];
        document.querySelectorAll('[data-i18n]').forEach(el => {
            const key = el.getAttribute('data-i18n');
            if (t[key]) {
                if (key === 'friendly_abilities') {
                    el.textContent = `${t[key]}: ${State.showFriendlyAbilities ? 'ON' : 'OFF'}`;
                } else {
                    el.textContent = t[key];
                }
            }
        });

        const btn = document.getElementById('lang-btn');
        if (btn) btn.textContent = State.currentLang === 'jp' ? 'English' : '日本語';

        if (State.data && window.render) window.render();
    },

    // --- Performance Modal ---
    showLastPerformance: () => {
        const modal = document.getElementById('performance-modal');
        if (modal) modal.style.display = 'flex';
        // By default show the latest results
        if (Rendering && Rendering.renderPerformanceResult) {
            const latestTurn = State.performanceHistoryTurns && State.performanceHistoryTurns.length > 0
                ? Math.max(...State.performanceHistoryTurns)
                : -1;
            State.selectedPerfTurn = latestTurn;
            const dataToUse = latestTurn !== -1 ? State.performanceHistory[latestTurn] : State.lastPerformanceData;
            Rendering.renderPerformanceResult(dataToUse);
        }
    },

    showPerformanceForTurn: (turn) => {
        State.selectedPerfTurn = turn;
        const data = State.performanceHistory[turn];
        if (data && Rendering && Rendering.renderPerformanceResult) {
            Rendering.renderPerformanceResult(data);
        }
    },

    closePerformanceModal: () => {
        const modal = document.getElementById('performance-modal');
        if (modal) modal.style.display = 'none';
    },

    // --- Help Modal ---
    openHelpModal: () => {
        const modal = document.getElementById('help-modal');
        if (modal) modal.style.display = 'flex';
    },

    closeHelpModal: () => {
        const modal = document.getElementById('help-modal');
        if (modal) modal.style.display = 'none';
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
            Modals.closeDeckModal();
            if (window.fetchState) window.fetchState();
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
            Modals.closeDeckModal();
            if (window.fetchState) window.fetchState();
        } catch (e) {
            console.error(e);
            alert("Error loading random deck: " + e.message);
        }
    },

    // --- Lobby Modal ---
    openLobby: () => {
        const modal = document.getElementById('room-modal');
        if (modal) modal.style.display = 'flex';
        if (window.fetchPublicRooms) window.fetchPublicRooms();
    },

    closeLobby: () => {
        const modal = document.getElementById('room-modal');
        if (modal) modal.style.display = 'none';
    },

    // --- Report Modal ---
    openReportModal: () => {
        const modal = document.getElementById('report-modal');
        if (modal) modal.style.display = 'flex';
    },

    closeReportModal: () => {
        const modal = document.getElementById('report-modal');
        if (modal) modal.style.display = 'none';
    },

    submitReport: async () => {
        const explanation = document.getElementById('report-explanation').value;
        if (!explanation) {
            alert("Please provide an explanation of the issue.");
            return;
        }

        const success = await Network.submitReport(explanation);
        if (success) {
            alert("Report submitted successfully! Thank you for your feedback.");
            document.getElementById('report-explanation').value = "";
            Modals.closeReportModal();
        } else {
            alert("Failed to submit report. You can still 'Download JSON' to save it manually.");
        }
    },

    downloadReport: () => {
        const explanation = document.getElementById('report-explanation').value;
        const reportData = Network._buildSlimReport(explanation);
        // Add userAgent for local downloads (useful for debugging rendering issues)
        reportData.userAgent = navigator.userAgent;

        const blob = new Blob([JSON.stringify(reportData, null, 2)], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        const ts = new Date().toISOString().replace(/[:.]/g, '-');
        a.download = `lovecasim_report_${ts}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }
};
