import { State } from '../state.js';
import { Network } from '../network.js';
import { Modals } from '../ui_modals.js';
import { ModalManager } from '../utils/ModalManager.js';
import { DOM_IDS, DISPLAY_VALUES } from '../constants_dom.js';

function normalizeCode(code) {
    if (!code) return '';
    return code.replace(/＋/g, '+').replace(/－/g, '-').replace(/ー/g, '-').trim().toUpperCase();
}

function extractCardId(title) {
    const parts = title.split(/\s*:\s*/);
    return normalizeCode(parts[0]);
}

function convertDecklogHtml(html) {
    const cards = {};
    let foundAny = false;

    // Pattern 1: DeckLog HTML <span title="ID : Name">...<span class="num">QTY</span>
    const decklogRe = /<span\s+title="([^"]+)"[^>]*>\s*<\/span>\s*<span[^>]*class="num"[^>]*>(\d+)<\/span>/g;
    let m;
    while ((m = decklogRe.exec(html)) !== null) {
        const cardId = extractCardId(m[1]);
        const qty = parseInt(m[2], 10);
        if (cardId && qty > 0) {
            cards[cardId] = (cards[cardId] || 0) + qty;
            foundAny = true;
        }
    }

    // Pattern 2: Official Love Live site deck recipe format
    //   <a href="/cardlist/searchresults/?cardno=PL!N-bp3-030-L">
    //     <img ...><span class="sheet"><span>×</span><span>2</span></span></a>
    if (!foundAny) {
        const officialRe = /<a[^>]*href="[^">]*cardno=([^"&>]+)[^">]*"[\s\S]*?<span class="sheet"><span>[×xX]<\/span><span>(\d+)<\/span><\/span><\/a>/gi;
        while ((m = officialRe.exec(html)) !== null) {
            const cardId = normalizeCode(m[1]);
            const qty = parseInt(m[2], 10);
            if (cardId && qty > 0) {
                cards[cardId] = (cards[cardId] || 0) + qty;
                foundAny = true;
            }
        }
    }

    // Pattern 3: plain text lines "ID x Count"
    if (!foundAny) {
        const lines = html.split(/\r?\n/);
        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed || trimmed.startsWith('#')) continue;
            const xMatch = trimmed.match(/^(.+?)\s*[xX×]\s*(\d+)$/);
            if (xMatch) {
                const id = normalizeCode(xMatch[1]);
                const qty = parseInt(xMatch[2], 10);
                if (id && qty > 0) {
                    cards[id] = (cards[id] || 0) + qty;
                    foundAny = true;
                }
            } else {
                const id = normalizeCode(trimmed);
                if (id && id.includes('-')) {
                    cards[id] = (cards[id] || 0) + 1;
                    foundAny = true;
                }
            }
        }
    }

    if (!foundAny || Object.keys(cards).length === 0) return null;

    // Convert to flat list (one card_no per entry)
    const result = [];
    for (const [cardNo, qty] of Object.entries(cards)) {
        for (let i = 0; i < qty; i++) result.push(cardNo);
    }
    return result;
}

function setupAutoConvert(pid) {
    const textarea = document.getElementById(`p${pid}-deck-paste`);
    const status = document.getElementById(`p${pid}-convert-status`);
    if (!textarea || !status) return;

    // Add "Load from file" button for this paste area
    const pasteContainer = textarea.closest('.setup-paste-container');
    if (pasteContainer && !pasteContainer.querySelector('.deck-file-load-btn')) {
        const loadBtn = document.createElement('button');
        loadBtn.className = 'btn btn-small deck-file-load-btn';
        loadBtn.textContent = '📂 Load from file';
        loadBtn.type = 'button';
        loadBtn.style.marginTop = '6px';
        loadBtn.style.fontSize = '0.8rem';
        loadBtn.addEventListener('click', () => {
            const fileInput = document.createElement('input');
            fileInput.type = 'file';
            fileInput.accept = '.html,.txt,.json';
            fileInput.style.display = 'none';
            fileInput.addEventListener('change', async () => {
                if (fileInput.files[0]) {
                    try {
                        const content = await fileInput.files[0].text();
                        textarea.value = content;
                        status.textContent = `Loaded: ${fileInput.files[0].name}`;
                        status.style.color = '#22c55e';
                        textarea.dispatchEvent(new Event('input'));
                    } catch (e) {
                        status.textContent = 'Failed to read file';
                        status.style.color = '#ef4444';
                    }
                }
                fileInput.remove();
            });
            document.body.appendChild(fileInput);
            fileInput.click();
        });
        pasteContainer.appendChild(loadBtn);
    }

    let timeout = null;
    textarea.addEventListener('input', () => {
        clearTimeout(timeout);
        timeout = setTimeout(() => {
            const val = textarea.value.trim();
            if (!val) {
                status.textContent = '';
                return;
            }
            // Only attempt HTML conversion if it looks like HTML (decklog or official site format)
            if (val.includes('<span') || val.includes('title=') || val.includes('class="num"') || val.includes('cardno=')) {
                const deck = convertDecklogHtml(val);
                if (deck && deck.length > 0) {
                    textarea.value = deck.join('\n');
                    status.textContent = `Converted: ${deck.length} cards`;
                    status.style.color = '#22c55e';
                } else {
                    status.textContent = 'Could not parse HTML';
                    status.style.color = '#ef4444';
                }
            } else {
                // Plain text — count lines
                const lines = val.split(/\r?\n/).filter(l => l.trim() && !l.startsWith('#'));
                let total = 0;
                for (const line of lines) {
                    const xMatch = line.match(/^(.+?)\s*[xX×]\s*(\d+)$/);
                    total += xMatch ? parseInt(xMatch[2], 10) : 1;
                }
                status.textContent = `${lines.length} types, ${total} cards`;
                status.style.color = '';
            }
        }, 400);
    });
}

export const GameSetupModal = {
    openSetupModal: (mode) => {
        ModalManager.show(DOM_IDS.MODAL_SETUP);
        ModalManager.hide(DOM_IDS.MODAL_ROOM);

        Modals.fetchAndPopulateDecks().then(() => {
            Modals.populateDeckSelect('p0-deck-select', Modals.deckPresets);
            Modals.populateDeckSelect('p1-deck-select', Modals.deckPresets);
        });

        const p0Col = document.getElementById('setup-p0-col');
        const p1Col = document.getElementById('setup-p1-col');
        const title = document.getElementById('setup-title');
        const roomCodeEl = document.getElementById('setup-room-code');
        if (title) title.textContent = 'Sandbox Setup';
        if (roomCodeEl) {
            roomCodeEl.style.display = DISPLAY_VALUES.NONE;
            roomCodeEl.textContent = '';
        }

        if (p0Col) p0Col.style.display = DISPLAY_VALUES.BLOCK;
        if (p1Col) {
            p1Col.style.display = DISPLAY_VALUES.BLOCK;
            p1Col.style.opacity = '1';
            p1Col.style.pointerEvents = 'auto';
            const p1Title = p1Col.querySelector('h4');
            if (p1Title) p1Title.textContent = 'Player 2 (AI)';
        }

        // Setup auto-convert for both paste areas
        setupAutoConvert(0);
        setupAutoConvert(1);
    },

    closeSetupModal: () => {
        ModalManager.hide(DOM_IDS.MODAL_SETUP);
        Modals.pvpJoinPid = null;
        // Only return to lobby if a game hasn't started yet
        if (!State.gameHasStarted) {
            ModalManager.show(DOM_IDS.MODAL_ROOM);
        }
    },

    getDeckConfig: (pid) => {
        const selectId = `p${pid}-deck-select`;
        const select = document.getElementById(selectId);
        if (!select) return null;

        const mode = select.value;
        if (mode === 'manual' || mode === 'paste') {
            const input = document.getElementById(`p${pid}-deck-paste`);
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
            const content = config.content || '';
            if (!content.trim()) {
                console.warn("Manual deck is empty");
                return { main: [], energy: [] };
            }
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
        if (Modals.pvpJoinPid !== undefined && Modals.pvpJoinPid !== null) {
            await GameSetupModal.submitPvPDeck();
            Modals.pvpJoinPid = null;
            return;
        }

        if (!State.roomCode) {
            const roomRes = await fetch('api/rooms/create', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ mode: 'sandbox' })
            });
            const roomData = await roomRes.json();
            if (roomData.success) {
                State.roomCode = roomData.room_id;
                if (roomData.session && Network?.saveSession) {
                    Network.saveSession(roomData.room_id, roomData.session);
                }
            }
        }

        const p0Config = GameSetupModal.getDeckConfig(0);
        const p1Config = GameSetupModal.getDeckConfig(1);

        try {
            const p0Deck = await GameSetupModal.resolveDeck(p0Config);
            const p1Deck = await GameSetupModal.resolveDeck(p1Config);

            if (!p0Deck || !p1Deck) {
                alert("Failed to resolve decks. Please check console.");
                return;
            }

            const headers = Network?.getHeaders ? Network.getHeaders() : { 'Content-Type': 'application/json' };

            await Promise.all([
                fetch('api/set_deck', {
                    method: 'POST', headers,
                    body: JSON.stringify({ player: 0, deck: p0Deck.main, room_id: State.roomCode })
                }),
                fetch('api/set_deck', {
                    method: 'POST', headers,
                    body: JSON.stringify({ player: 1, deck: p1Deck.main, room_id: State.roomCode })
                })
            ]);

            const initRes = await fetch('api/init', {
                method: 'POST', headers,
                body: JSON.stringify({})
            });

            if (!initRes.ok) {
                const errorData = await initRes.json().catch(() => ({ error: "Server error" }));
                throw new Error(errorData.error || `HTTP error! status: ${initRes.status}`);
            }

            const data = await initRes.json();
            State.offlineMode = false;

            ModalManager.hide(DOM_IDS.MODAL_ROOM);
            ModalManager.hide(DOM_IDS.MODAL_SETUP);
            Modals.pvpJoinPid = null;
            await Network.fetchState();
        } catch (e) {
            console.error(e);
            alert("Network error: " + e.message);
        }
    },

    openDeckSelectionForPvP: (pid) => {
        Modals.pvpJoinPid = pid;
        ModalManager.hide(DOM_IDS.MODAL_ROOM);
        ModalManager.show(DOM_IDS.MODAL_SETUP);

        const p0Col = document.getElementById('setup-p0-col');
        const p1Col = document.getElementById('setup-p1-col');
        const startBtn = document.getElementById('setup-start-btn');
        const title = document.getElementById('setup-title');
        const roomCodeEl = document.getElementById('setup-room-code');

        if (title) title.textContent = 'Select Your Deck';
        if (roomCodeEl && State.roomCode) {
            roomCodeEl.textContent = `Room: ${State.roomCode}`;
            roomCodeEl.style.display = 'block';
        }

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
            startBtn.setAttribute('data-action', 'submit-game-setup');
        }

        Modals.fetchAndPopulateDecks().then(() => {
            const selectId = pid === 0 ? 'p0-deck-select' : 'p1-deck-select';
            Modals.populateDeckSelect(selectId, Modals.deckPresets);
        });

        // Setup auto-convert for this player's paste area
        setupAutoConvert(pid);
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
                    energy_deck: resolved.energy,
                    room_id: State.roomCode
                })
            });
            const data = await res.json();
            if (data.success || data.status === 'ok') {
                if (data.room_init) {
                    ModalManager.hide(DOM_IDS.MODAL_SETUP);
                    Modals.pvpJoinPid = null;
                    ModalManager.hide(DOM_IDS.MODAL_ROOM);
                    await Network.fetchState();
                } else {
                    const startBtn = document.querySelector('[data-action="submit-game-setup"]');
                    if (startBtn) {
                        startBtn.textContent = 'Waiting for opponent...';
                        startBtn.disabled = true;
                    }
                    // Poll for game state every 3s as fallback for SSE
                    // Cloudflared and some proxies buffer SSE streams, so the
                    // host may never receive the "update" event via SSE.
                    const pollInterval = setInterval(async () => {
                        // Stop polling if modals are already closed (game started)
                        const setup = document.getElementById(DOM_IDS.MODAL_SETUP);
                        if (!setup || setup.style.display === 'none') {
                            clearInterval(pollInterval);
                            return;
                        }
                        await Network.fetchState();
                        // If game started, fetchState closes the modals and
                        // the next poll will stop itself.
                    }, 3000);
                    // Store reference for cleanup
                    window._pvpPollInterval = pollInterval;
                }
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
        const pasteArea = document.getElementById(`p${pid}-paste-area`);
        if (pasteArea) {
            pasteArea.style.display = (finalValue === 'paste' || finalValue === 'manual') ? DISPLAY_VALUES.BLOCK : DISPLAY_VALUES.NONE;
        }
    }
};
