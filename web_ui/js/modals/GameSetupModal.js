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

let _ciIndex = null;
function _ensureIndex() {
    const db = State.staticCardDatabase;
    if (!db) return;
    if (_ciIndex) return;
    _ciIndex = {};
    for (const key of Object.keys(db)) {
        const nk = key.replace(/＋/g, '+').toUpperCase();
        _ciIndex[nk] = key;
    }
}

function lookupCard(no) {
    const db = State.staticCardDatabase;
    if (!db) return null;
    let card = db[no];
    if (card) return card;
    _ensureIndex();
    const nk = no.replace(/＋/g, '+').toUpperCase();
    const actualKey = _ciIndex?.[nk];
    return actualKey ? db[actualKey] : null;
}

function parsePointSectionFormat(text) {
    let members = 0, lives = 0, energy = 0, points = 0;
    let currentPt = 0;
    const lines = text.split(/\r?\n/);
    for (const line of lines) {
        const t = line.trim();
        if (!t || t.startsWith('#')) continue;
        const ptMatch = t.match(/^(\d+)ポイント/);
        if (ptMatch) { currentPt = parseInt(ptMatch[1], 10); continue; }
        if (/^テキスト\d+種/.test(t)) continue;
        const first = t.split(/\s+/)[0];
        if (first.startsWith('(') || !first.includes('-')) continue;
        const card = lookupCard(first);
        const type = card?.type || '';
        if (type === 'メンバー') members++;
        else if (type === 'ライブ') lives++;
        else if (type === 'エネルギー') energy++;
        points += currentPt || 1;
    }
    return { members, lives, energy, points };
}

const _POINT_MAP = {
    'PL!N-bp1-003-R+': 4, 'PL!N-bp1-003-P': 4, 'PL!N-bp1-003-P＋': 4, 'PL!N-bp1-003-SEC': 4,
    'PL!N-bp1-012-R+': 3, 'PL!N-bp1-012-P': 3, 'PL!N-bp1-012-P＋': 3, 'PL!N-bp1-012-SEC': 3,
    'LL-bp2-001-R+': 3, 'LL-bp2-001-R＋': 3,
    'PL!N-bp1-002-R+': 2, 'PL!N-bp1-002-P': 2, 'PL!N-bp1-002-P＋': 2, 'PL!N-bp1-002-SEC': 2,
    'PL!N-sd1-008-SD': 2, 'PL!N-sd1-008-RM': 2, 'PL!HS-bp2-014-N': 2,
    'PL!SP-bp1-005-R': 1, 'PL!SP-bp1-005-P': 1, 'PL!N-bp1-029-L': 1,
    'PL!SP-sd1-019-SD': 1, 'PL!SP-sd1-019-RM': 1,
    'PL!SP-sd1-020-SD': 1, 'PL!SP-sd1-020-RM': 1,
    'PL!SP-pb1-014-N': 1, 'PL!SP-bp2-024-L': 1, 'PL!SP-bp2-024-SECL': 1,
};
let _ptCI = null;
function cardPoints(no) {
    let p = _POINT_MAP[no];
    if (p !== undefined) return p;
    if (!_ptCI) {
        _ptCI = {};
        for (const [k, v] of Object.entries(_POINT_MAP)) {
            _ptCI[k.replace(/＋/g, '+').toUpperCase()] = v;
        }
    }
    const nk = no.replace(/＋/g, '+').toUpperCase();
    return _ptCI[nk] ?? 0;
}

function parseSimpleCardLines(text) {
    let members = 0, lives = 0, energy = 0, points = 0, unknown = 0;
    const lines = text.split(/\r?\n/);
    for (const line of lines) {
        const t = line.trim();
        if (!t || t.startsWith('#')) continue;
        let cardNo = '', qty = 1;
        const qtyFirst = t.match(/^(\d+)\s*[xX×]\s*(.+)$/);
        const qtyLast = t.match(/^(.+?)\s*[xX×]\s*(\d+)$/);
        if (qtyFirst) { qty = parseInt(qtyFirst[1], 10); cardNo = qtyFirst[2].trim(); }
        else if (qtyLast) { cardNo = qtyLast[1].trim(); qty = parseInt(qtyLast[2], 10); }
        else { cardNo = t; }
        if (!cardNo.includes('-')) continue;
        const card = lookupCard(cardNo);
        const type = card?.type || '';
        for (let i = 0; i < qty; i++) {
            if (type === 'メンバー') members++;
            else if (type === 'ライブ') lives++;
            else if (type === 'エネルギー') energy++;
            const pt = cardPoints(cardNo);
            if (pt > 0) points += pt;
            else unknown++;
        }
    }
    return { members, lives, energy, points, unknown };
}

function parseDeckText(val) {
    if (val.includes('<span') || val.includes('title=') || val.includes('class="num"') || val.includes('cardno=')) {
        const deck = convertDecklogHtml(val);
        if (deck && deck.length > 0) { const raw = deck.join('\n'); return { raw, analysis: parseSimpleCardLines(raw), converted: true }; }
        return null;
    }
    if (/\d+ポイント/.test(val)) {
        return { raw: val, analysis: parsePointSectionFormat(val), converted: false };
    }
    return { raw: val, analysis: parseSimpleCardLines(val), converted: false };
}

function updateStatus(status, val) {
    if (!val) { status.textContent = ''; return; }
    const result = parseDeckText(val);
    if (!result) {
        status.textContent = 'Could not parse';
        status.style.color = '#ef4444';
        return;
    }
    if (result.converted) {
        const textarea = status.parentElement?.querySelector('textarea');
        if (textarea) textarea.value = result.raw;
    }
    const a = result.analysis;
    const parts = [`M:${a.members}`, `L:${a.lives}`, `E:${a.energy}`];
    if (a.points > 0) parts.push(`P:${a.points}`);
    status.textContent = parts.join(' ');
    status.style.color = '#22c55e';
}

function setupAutoConvert(pid) {
    const textarea = document.getElementById(`p${pid}-deck-paste`);
    const status = document.getElementById(`p${pid}-convert-status`);
    if (!textarea || !status) return;

    const doUpdate = () => {
        if (textarea.value.trim()) updateStatus(status, textarea.value.trim());
    };
    if (State.staticCardDatabase) {
        doUpdate();
    } else {
        const _onDbReady = () => { State.off('carddb-loaded', _onDbReady); doUpdate(); };
        State.on('carddb-loaded', _onDbReady);
    }

    let timeout = null;
    textarea.addEventListener('input', () => {
        clearTimeout(timeout);
        timeout = setTimeout(() => updateStatus(status, textarea.value.trim()), 400);
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

        // Paste is now default — ensure paste areas are visible
        GameSetupModal.onDeckSelectChange(0, 'paste');
        GameSetupModal.onDeckSelectChange(1, 'paste');
        // Direct fallback in case onDeckSelectChange didn't set display properly
        const p0pa = document.getElementById('p0-paste-area');
        const p1pa = document.getElementById('p1-paste-area');
        if (p0pa) p0pa.style.display = 'block';
        if (p1pa) p1pa.style.display = 'block';
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

            const p0Content = (typeof p0Config?.content === 'string' ? p0Config.content : '').trim();
            const p1Content = (typeof p1Config?.content === 'string' ? p1Config.content : '').trim();
            State.deckAnalysis = {
                p0: /\d+ポイント/.test(p0Content) ? parsePointSectionFormat(p0Content) : parseSimpleCardLines(p0Content || p0Deck.main.join('\n')),
                p1: /\d+ポイント/.test(p1Content) ? parsePointSectionFormat(p1Content) : parseSimpleCardLines(p1Content || p1Deck.main.join('\n')),
            };

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
