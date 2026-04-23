/**
 * Network & API Layer
 * Handles all communication with the backend (HTTP & WebSocket/Polling) and Game State updates.
 */
import { State, updateStateData } from './state.js';
import { log } from './logger.js';
import { Phase, getAppBaseUrl } from './constants.js';

let onRender = () => { console.warn("Network: No render callback set"); };
let onRoomUpdate = () => { console.warn("Network: No room update callback set"); };
let onOpenDeckModal = () => { console.warn("Network: No deck modal callback set"); };

export const Network = {
    setRenderCallback: (cb) => { onRender = cb; },
    setRoomUpdateCallback: (cb) => { onRoomUpdate = cb; },
    setOpenDeckModalCallback: (cb) => { onOpenDeckModal = cb; },
    triggerRoomUpdate: () => { if (typeof onRoomUpdate === 'function') onRoomUpdate(); },

    // Helpers
    getHeaders: () => {
        const headers = { 'Content-Type': 'application/json' };
        if (State.roomCode) headers['X-Room-ID'] = State.roomCode;
        if (State.sessionToken) headers['X-Session-Token'] = State.sessionToken;
        if (State.currentLang) headers['X-Language'] = State.currentLang;
        console.log("[Network] Headers:", { room: State.roomCode, token: !!State.sessionToken, lang: State.currentLang });
        return headers;
    },

    checkSystemStatus: async () => {
        const badge = document.getElementById('system-status-badge');
        if (!badge) return;
        try {
            const res = await fetch('api/status');
            const data = await res.json();
            if (data.status === 'rust_server') {
                const cardCount = (data.members || 0) + (data.lives || 0);
                badge.textContent = cardCount > 0 ? `ONLINE: ${cardCount} Cards` : "ONLINE: 0 Cards (ERROR)";
                badge.style.background = cardCount > 100 ? '#2ecc71' : '#f59e0b';
                badge.title = `Members: ${data.members}, Lives: ${data.lives} | ID: ${data.instance_id}`;

                // Automatic Reset on Server Restart
                if (data.instance_id) {
                    const lastId = localStorage.getItem('lovelive_server_instance_id');
                    if (lastId && lastId !== String(data.instance_id)) {
                        console.warn("[Network] Server instance ID changed! Forcing local reset...");
                        localStorage.setItem('lovelive_server_instance_id', data.instance_id);
                        if (typeof window.forceReset === 'function') {
                            window.forceReset();
                        }
                    } else {
                        localStorage.setItem('lovelive_server_instance_id', data.instance_id);
                    }
                }

                return data;
            } else {
                badge.textContent = "UNKNOWN";
                badge.style.background = "#e74c3c";
            }
        } catch (e) {
            badge.textContent = "OFFLINE";
            badge.style.background = "#e74c3c";
        }
        return null;
    },

    // Session Management
    saveSession: (room, sessionData) => {
        if (!room) return;
        const key = `lovelive_session_${room}`;
        localStorage.setItem(key, JSON.stringify(sessionData));
        State.sessionToken = sessionData.token;
        if (sessionData.playerId !== undefined) State.perspectivePlayer = sessionData.playerId;
    },

    loadSession: (room) => {
        if (!room) return null;
        const key = `lovelive_session_${room}`;
        const saved = localStorage.getItem(key);
        if (saved) {
            try {
                const data = JSON.parse(saved);
                State.sessionToken = data.token;
                if (data.playerId !== undefined) State.perspectivePlayer = data.playerId;
                return data;
            } catch (e) {
                console.error("Failed to load session", e);
            }
        }
        return null;
    },

    // Room Management
    createRoom: async (mode = 'pve') => {
        try {
            const res = await fetch('api/rooms/create', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ mode: mode })
            });
            const data = await res.json();
            if (data.success) {
                State.roomCode = data.room_id;
                State.offlineMode = false;
                localStorage.setItem('lovelive_room_code', State.roomCode);

                // Clear old session for new room
                State.sessionToken = null;
                localStorage.removeItem(`lovelive_session_${State.roomCode}`);

                document.getElementById('room-modal').style.display = 'none';
                log(`Created Room: ${State.roomCode} (${mode})`);

                // Initial fetch
                await Network.fetchState();
                onRoomUpdate();

                // If PvP, we might be P0 waiting for P1
                if (mode === 'pvp') {
                    // Poll faster initially
                    setTimeout(() => Network.fetchState(), 1000);
                }
            } else {
                alert('Failed to create room: ' + data.error);
            }
        } catch (e) {
            console.error(e);
            alert('Network error creating room');
        }
    },

    joinRoom: async (code = null) => {
        if (!code) {
            const input = document.getElementById('room-code-input');
            code = input.value.toUpperCase();
        }
        if (!code || code.length !== 4) {
            alert('Please enter a 4-letter room code.');
            return;
        }
        State.roomCode = code;
        State.offlineMode = false;
        localStorage.setItem('lovelive_room_code', State.roomCode);

        // Try to load existing session first
        const session = Network.loadSession(code);

        // If we have a deck on the clipboard/textarea, we should probably submit it?
        // Explicitly join via API to register session
        try {
            const res = await fetch('api/rooms/join', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ room_id: code })
            });
            const data = await res.json();
            if (data.success) {
                Network.saveSession(code, { token: data.session, playerId: data.player_idx });
            } else {
                console.warn("Join failed or room full:", data.error);
                // We might still be able to spectate?
            }
        } catch (e) {
            console.error("Join API error", e);
        }

        document.getElementById('room-modal').style.display = 'none';
        log(`Joining Room: ${State.roomCode}...`);

        onRoomUpdate();
        await Network.fetchState();
    },

    // Core Game loop
    fetchState: async () => {
        try {
            // [TRACE] Start fetchState
            if (State.replayMode) {
                console.log("[Trace] fetchState exiting: replayMode=true");
                return;
            }

            if (State.offlineMode) {
                if (!State.wasmAdapter) {
                    console.log("[Trace] fetchState exiting: offlineMode=true but no wasmAdapter");
                    return;
                }
                const res = await State.wasmAdapter.fetchState();
                if (res.success) {
                    const raw = JSON.stringify(res.state);
                    if (raw === State.lastStateJson) return;
                    State.lastStateJson = raw;
                    updateStateData(res.state);
                    onRender();
                }
                return;
            }

            if (!State.roomCode) {
                console.log("[Trace] fetchState exiting: roomCode is falsy", State.roomCode);
                return;
            }

            const perfModal = document.getElementById('performance-modal');
            if (perfModal && (perfModal.style.display === 'flex' || perfModal.style.display === 'block')) {
                console.log("[Trace] fetchState paused: performance-modal is visible");
                return;
            }

            // Session check
            if (!State.sessionToken && State.roomCode) {
                Network.loadSession(State.roomCode);
            }

            const headers = Network.getHeaders();
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 2000);

            // Phase 6: Cache-Busting & Final Trace
            const viewer = State.perspectivePlayer;
            const timestamp = Date.now();
            const url = `api/state?viewer=${viewer}&_t=${timestamp}`;

            // [TRACE] Final point before fetch
            console.log(`[Trace] fetchState CALLING: ${url}`);

            const res = await fetch(url, {
                headers,
                signal: controller.signal
            });
            clearTimeout(timeoutId);

            if (res.status === 404) {
                if (State.roomCode) {
                    console.warn(`[Network] Room ${State.roomCode} not found (404). Resetting local state.`);
                    const codeToClear = State.roomCode;
                    localStorage.removeItem('lovelive_room_code');

                    // Clean up session storage for this room
                    try {
                        let sessions = JSON.parse(localStorage.getItem('lovelive_sessions') || '{}');
                        if (sessions[codeToClear]) {
                            delete sessions[codeToClear];
                            localStorage.setItem('lovelive_sessions', JSON.stringify(sessions));
                        }
                    } catch (e) {
                        console.error("[Network] Failed to clean up sessions:", e);
                    }

                    State.roomCode = null;
                    State.sessionToken = null;
                    updateStateData(null);

                    if (typeof onRoomUpdate === 'function') onRoomUpdate();

                    // Force lobby visibility
                    const roomModal = document.getElementById('room-modal');
                    if (roomModal) {
                        roomModal.style.display = 'flex';
                    } else {
                        // Fallback: If modal not found, maybe just reload?
                        window.location.reload();
                    }

                    log(`Room ${codeToClear} expired or not found.`, 'error');
                }
                return;
            }

            const raw = await res.text();
            if (raw === State.lastStateJson) return;

            State.lastStateJson = raw;
            const data = JSON.parse(raw);

            if (data.success) {
                updateStateData(data.state);

                // Auto-sync perspective
                if (State.data.my_player_id !== undefined && State.data.my_player_id !== -1 && !State.hotseatMode) {
                    State.perspectivePlayer = State.data.my_player_id;
                }

                // Needs deck check
                if (State.data.needs_deck && !State.hotseatMode && !State.offlineMode) {
                    console.log("[Network] needs_deck detected. triggering onOpenDeckModal.");
                    // We need to trigger deck modal
                    // We can check if setup-modal is open
                    const setupModal = document.getElementById('setup-modal');
                    if (setupModal && setupModal.style.display !== 'flex') {
                        console.log("[Network] needs_deck detected.");
                        onOpenDeckModal(State.perspectivePlayer);
                    }
                }
            } else {
                console.error("State fetch unsuccessful:", data.error);
                return;
            }

            // Performance history tracking
            const turnNum = State.data.turn_number || State.data.turn || 0;
            const perfRes = State.data.performance_results || State.data.last_performance_results;

            if (perfRes && Object.keys(perfRes).length > 0) {
                State.lastPerformanceData = perfRes;
            }

            // Sync history from backend if available
            if (State.data.performance_history && Array.isArray(State.data.performance_history)) {
                State.data.performance_history.forEach(item => {
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
            } else if (perfRes && Object.keys(perfRes).length > 0) {
                // Fallback: local tracking if backend history is empty (e.g. legacy or during transition)
                if (!State.performanceHistory[turnNum]) {
                    State.performanceHistory[turnNum] = perfRes;
                    if (!State.performanceHistoryTurns.includes(turnNum)) {
                        State.performanceHistoryTurns.push(turnNum);
                        State.performanceHistoryTurns.sort((a, b) => b - a);
                    }
                }
            }

            onRender();
        } catch (e) {
            console.error("Fetch Error:", e);
            // If we have a room code but the fetch failed (network/server down),
            // and it's not a timeout, we should probably return to lobby or alert.
            if (e.name === 'AbortError') {
                console.warn("[Network] Fetch state timed out.");
            } else {
                if (State.roomCode) {
                    console.error("[Network] Critical connection failure. Returning to lobby.");
                    State.roomCode = null;
                    State.sessionToken = null;
                    localStorage.removeItem('lovelive_room_code');
                    updateStateData(null);
                    if (typeof onRoomUpdate === 'function') onRoomUpdate();

                    const roomModal = document.getElementById('room-modal');
                    if (roomModal) roomModal.style.display = 'flex';
                    log("Connection lost or server unreachable.", 'error');
                }
            }
        }
    },

    sendAction: async (id) => {
        // RPS phase logic: check active player?
        // Main.js logic:
        // const isRpsPhase = state && state.phase === Phase.RPS;
        // if (!isRpsPhase && state.active_player !== perspectivePlayer && !hotseatMode) ...

        const state = State.data;
        const isRpsPhase = state && state.phase === Phase.RPS;

        if (!isRpsPhase && state.active_player !== State.perspectivePlayer && !State.hotseatMode) {
            console.warn("[Network] Action blocked: Not your turn.");
            return;
        }

        if (window.pendingAction) return; // Use window for now or move to State?
        window.pendingAction = true;
        document.body.classList.add('action-pending');
        log(`Action: ${id}`, 'action');

        try {
            if (State.offlineMode) {
                const res = await State.wasmAdapter.doAction(id);
                if (res.success) {
                    updateStateData(res.state);
                    State.lastStateJson = JSON.stringify(res.state);
                    onRender();
                    log('Action completed');
                } else {
                    alert(res.error);
                }
                return;
            }

            const res = await fetch('api/action', {
                method: 'POST',
                headers: Network.getHeaders(),
                body: JSON.stringify({ action_id: id })
            });
            const text = await res.text();
            State.lastStateJson = text;
            const data = JSON.parse(text);

            if (data.success) {
                updateStateData(data.state);
                if (State.data.my_player_id !== undefined && State.data.my_player_id !== -1 && !State.hotseatMode) {
                    State.perspectivePlayer = State.data.my_player_id;
                }
                onRender();
                log('Action completed');
                // adaptive polling logic? omitted for brevity or passed in?
            } else {
                alert(data.error || 'Unknown error');
            }
        } finally {
            window.pendingAction = false;
            document.body.classList.remove('action-pending');
        }
    },

    resetGame: async () => {
        log('Resetting game...');

        if (State.offlineMode) {
            const res = await State.wasmAdapter.resetGame();
            if (res.success) {
                updateStateData(res.state);
                window.lastShownPerformanceHash = "";
                onRender();
                log('New game started');
            }
            return;
        }

        const state = State.data;
        const modeToUse = (state && state.mode) ? state.mode : (State.hotseatMode ? "pvp" : "pve");

        try {
            const res = await fetch('api/reset', {
                method: 'POST',
                headers: Network.getHeaders(),
                body: JSON.stringify({ mode: modeToUse })
            });

            if (!res.ok) {
                const txt = await res.text();
                log(`Reset failed: ${res.status}`);
                console.error(txt);
                return;
            }

            const text = await res.text();
            State.lastStateJson = text;
            const data = JSON.parse(text);

            if (data.success) {
                updateStateData(data.state);
                window.lastShownPerformanceHash = "";
                onRender();
                log('New game started');
                await Network.fetchState();
            } else {
                log(`Reset failed: ${data.error}`);
            }
        } catch (e) {
            log(`Reset error: ${e.message}`);
        }
    },

    changeAI: async (aiMode) => {
        console.log(`Switching AI to ${aiMode}`);
        try {
            const res = await fetch('api/set_ai', {
                method: 'POST',
                headers: Network.getHeaders(),
                body: JSON.stringify({ ai_mode: aiMode })
            });
            const data = await res.json();
            if (data.success) console.log(`AI switched to ${aiMode}`);
            else alert('Failed: ' + data.error);
        } catch (e) {
            console.error(e);
        }
    },

    forceAction: async (id) => {
        console.log(`Force action: ${id}`);
        const res = await fetch('api/action', {
            method: 'POST',
            headers: Network.getHeaders(),
            body: JSON.stringify({ action_id: id, force: true })
        });
        const text = await res.text();
        State.lastStateJson = text;
        const data = JSON.parse(text);
        if (data.success) {
            updateStateData(data.state);
            onRender();
            console.log('Forced');
        } else {
            console.log('Error: ' + data.error);
        }
    },

    forcedTurnEnd: async () => {
        await fetch('api/force_turn_end', { method: 'POST', headers: Network.getHeaders() });
        await Network.fetchState();
    },

    execCode: async (code) => {
        console.log(`Exec: ${code.substring(0, 30)}...`);
        const res = await fetch('api/exec', {
            method: 'POST',
            headers: Network.getHeaders(),
            body: JSON.stringify({ code: code })
        });
        const text = await res.text();
        State.lastStateJson = text;
        const data = JSON.parse(text);
        if (data.success) {
            updateStateData(data.state);
            onRender();
            log('Code executed');
        } else {
            log('Error: ' + data.error);
        }
    },

    startOffline: async (userInitiated = true) => {
        if (userInitiated) {
            const confirmMsg = "Offline mode runs entirely in your browser using WebAssembly.\n\n" +
                "It may take a moment to load the engine.";
            if (!confirm(confirmMsg)) return;
        }

        try {
            // Dynamically load WASM Adapter if not present
            if (!State.wasmAdapter) {
                try {
                    console.log("[OFFLINE] Importing WASM Adapter...");
                    const base = getAppBaseUrl();
                    const mod = await import(`${base}js/wasm_adapter.js`);
                    State.wasmAdapter = mod.wasmAdapter;
                    await State.wasmAdapter.init();
                } catch (e) {
                    console.error("Failed to load WASM:", e);
                    alert("Failed to load Offline Engine: " + e.message);
                    return;
                }
            }


            State.offlineMode = true;
            State.roomCode = null;
            State.sessionToken = null;
            updateStateData(null);

            document.getElementById('room-modal').style.display = 'none';
            document.getElementById('header-debug-info').textContent = "Offline (WASM)";

            onRoomUpdate();

            log("Starting Offline Game...");

            // Initialize game
            const res = await State.wasmAdapter.resetGame();
            if (res.success) {
                updateStateData(res.state);
                onRender();
                log("Offline Game Started!");
            } else {
                alert("Failed to start offline game: " + res.error);
            }
        } catch (e) {
            console.error(e);
            alert("Offline mode error: " + e.message);
        }
    },

    leaveRoom: () => {
        State.roomCode = null;
        State.sessionToken = null;
        updateStateData(null);
        localStorage.removeItem('lovelive_room_code');
        document.getElementById('room-modal').style.display = 'flex';
        // Cleanup URL if needed
        history.pushState({}, document.title, window.location.pathname);
        onRoomUpdate();
    },

    fetchPublicRooms: async () => {
        const list = document.getElementById('public-rooms-list');
        list.innerHTML = '<div style="color:#666;text-align:center;padding-top:20px;">Loading...</div>';

        try {
            const res = await fetch('api/rooms/list');
            const data = await res.json();

            if (!data.rooms || data.rooms.length === 0) {
                list.innerHTML = '<div style="color:#666;text-align:center;padding-top:20px;">No active public rooms.</div>';
                return;
            }

            list.innerHTML = '';
            data.rooms.forEach(r => {
                const div = document.createElement('div');
                div.style.padding = '5px';
                div.style.borderBottom = '1px solid #444';
                div.style.cursor = 'pointer';
                div.style.display = 'flex';
                div.style.justifyContent = 'space-between';
                div.innerHTML = `
                        <span>Room <b>${r.id}</b> (${r.mode})</span>
                        <span style="color:#aaa; font-size:0.8rem;">${r.players_count}/2</span>
                     `;
                div.onclick = () => {
                    document.getElementById('room-code-input').value = r.id;
                    Network.joinRoom(r.id);
                };
                list.appendChild(div);
            });
        } catch (e) {
            list.innerHTML = '<div style="color:#e74c3c;text-align:center;padding-top:20px;">Failed to load rooms.</div>';
        }
    },

    /**
     * Build a slim, readable report from the current game state.
     * Strips duplicates, hidden card arrays, img paths, and action history bloat.
     */
    _buildSlimReport: (explanation) => {
        const raw = State.data;
        if (!raw) return { explanation, timestamp: new Date().toISOString(), error: "No game state available" };

        // --- Helper: strip img & empty arrays from a card object ---
        const slimCard = (c) => {
            if (!c) return null;
            const out = { ...c };
            delete out.img;
            delete out.valid_actions;
            delete out.is_new;
            // Remove duplicate description (keep desc only)
            if (out.description && out.desc) delete out.description;
            // Compact heart arrays: strip trailing zeros -> e.g. [0,1,0,0,0,0,0] -> [0,1]
            for (const key of ['hearts', 'blade_hearts', 'required_hearts', 'filled', 'required']) {
                if (Array.isArray(out[key])) {
                    let last = out[key].length - 1;
                    while (last > 0 && out[key][last] === 0) last--;
                    out[key] = out[key].slice(0, last + 1);
                    if (out[key].length === 1 && out[key][0] === 0) out[key] = [];
                }
            }
            return out;
        };

        // --- Helper: slim a player object ---
        const slimPlayer = (p) => {
            if (!p) return null;
            const out = {};
            out.player_id = p.player_id;
            out.score = p.score;
            out.is_active = p.is_active;
            out.deck_count = p.deck_count;
            out.hand_count = p.hand_count;
            out.discard_count = p.discard_count;
            out.energy_count = p.energy_count;
            out.energy_untapped = p.energy_untapped;
            out.energy_deck_count = p.energy_deck_count;
            out.live_zone_count = p.live_zone_count;
            out.total_blades = p.total_blades;
            if (p.total_hearts) out.total_hearts = p.total_hearts;
            if (p.restrictions && p.restrictions.length > 0) out.restrictions = p.restrictions;

            // Preserve effect-related fields for Active Effects UI
            if (p.cost_reduction !== undefined) out.cost_reduction = p.cost_reduction;
            if (p.blade_buffs) out.blade_buffs = p.blade_buffs;
            if (p.heart_buffs) out.heart_buffs = p.heart_buffs;
            if (p.prevent_activate !== undefined) out.prevent_activate = p.prevent_activate;
            if (p.prevent_baton_touch !== undefined) out.prevent_baton_touch = p.prevent_baton_touch;
            if (p.prevent_success_pile_set !== undefined) out.prevent_success_pile_set = p.prevent_success_pile_set;
            if (p.played_group_mask !== undefined) out.played_group_mask = p.played_group_mask;
            if (p.yell_cards) out.yell_cards = p.yell_cards;
            if (p.heart_req_reductions) out.heart_req_reductions = p.heart_req_reductions;
            if (p.heart_req_additions) out.heart_req_additions = p.heart_req_additions;

            // Hand: show full cards only for the viewing player, counts for opponent
            if (Array.isArray(p.hand)) {
                const hasHidden = p.hand.some(c => c && c.hidden);
                if (hasHidden) {
                    // Opponent hand — just count is enough
                    // (hand_count already covers this)
                } else {
                    out.hand = p.hand.map(slimCard).filter(Boolean);
                }
            }

            // Stage
            if (Array.isArray(p.stage)) {
                out.stage = p.stage.map(s => s ? slimCard(s) : null);
            }

            // Live Zone
            if (Array.isArray(p.live_zone)) {
                const hasLives = p.live_zone.some(l => l !== null);
                if (hasLives) out.live_zone = p.live_zone.map(l => l ? slimCard(l) : null);
            }

            // Success Lives
            if (Array.isArray(p.success_lives) && p.success_lives.length > 0) {
                out.success_lives = p.success_lives.map(slimCard);
            }

            // Discard — include full cards for debugging context
            if (Array.isArray(p.discard) && p.discard.length > 0) {
                out.discard = p.discard.map(slimCard);
            }

            // Energy — only show tapped/untapped counts, not hidden card objects
            if (Array.isArray(p.energy)) {
                const tapped = p.energy.filter(e => e && e.tapped).length;
                const untapped = p.energy.filter(e => e && !e.tapped).length;
                out.energy_summary = { tapped, untapped };
            }

            return out;
        };

        // --- Helper: slim performance data ---
        const slimPerf = (perf) => {
            if (!perf) return null;
            const out = { ...perf };
            // Slim nested cards
            if (out.member_contributions) {
                out.member_contributions = out.member_contributions.map(mc => {
                    const s = { ...mc };
                    // Compact hearts
                    if (Array.isArray(s.hearts)) {
                        let last = s.hearts.length - 1;
                        while (last > 0 && s.hearts[last] === 0) last--;
                        s.hearts = s.hearts.slice(0, last + 1);
                        if (s.hearts.length === 1 && s.hearts[0] === 0) s.hearts = [];
                    }
                    return s;
                });
            }
            if (out.lives) out.lives = out.lives.map(slimCard);
            if (out.yell_cards) {
                out.yell_cards = out.yell_cards.map(yc => {
                    const s = { ...yc };
                    delete s.img;
                    return s;
                });
            }
            if (out.breakdown) {
                if (out.breakdown.hearts) {
                    out.breakdown.hearts = out.breakdown.hearts.map(h => {
                        const s = { ...h };
                        if (Array.isArray(s.value)) {
                            let last = s.value.length - 1;
                            while (last > 0 && s.value[last] === 0) last--;
                            s.value = s.value.slice(0, last + 1);
                            if (s.value.length === 1 && s.value[0] === 0) s.value = [];
                        }
                        return s;
                    });
                }
            }
            // Compact total_hearts
            if (Array.isArray(out.total_hearts)) {
                let last = out.total_hearts.length - 1;
                while (last > 0 && out.total_hearts[last] === 0) last--;
                out.total_hearts = out.total_hearts.slice(0, last + 1);
                if (out.total_hearts.length === 1 && out.total_hearts[0] === 0) out.total_hearts = [];
            }
            return out;
        };

        // === Build the slim report ===
        const report = {
            timestamp: new Date().toISOString(),
            explanation: explanation || "",
            room_id: State.roomCode || null,
            mode: raw.mode || "unknown",
            turn: raw.turn,
            phase: raw.phase,
            active_player: raw.active_player,
            winner: raw.winner,
            game_over: raw.game_over,
            my_player_id: raw.my_player_id
        };

        // Pending choice (important for bug reports)
        if (raw.pending_choice) report.pending_choice = raw.pending_choice;

        // Legal actions (current ones only, slim)
        if (Array.isArray(raw.legal_actions) && raw.legal_actions.length > 0) {
            report.legal_actions = raw.legal_actions.map(a => {
                const s = { ...a };
                delete s.description; // keep desc only
                delete s.img;
                return s;
            });
        }

        // Players (slimmed)
        if (Array.isArray(raw.players)) {
            report.players = raw.players.map(slimPlayer);
        }

        // Rule log (keep — it's essential for debugging)
        if (Array.isArray(raw.rule_log)) {
            report.rule_log = raw.rule_log;
        }

        // Performance history — use top-level only (avoid state.performance_history duplication)
        const perfHist = raw.performance_history || [];
        if (perfHist.length > 0) {
            report.performance_history = perfHist.map(slimPerf);
        }

        // Compact action history: just [timestamp, action_id, phase] tuples
        if (State.actionHistory && State.actionHistory.length > 0) {
            report.action_history = State.actionHistory;
        } else if (raw.history && Array.isArray(raw.history)) {
            // Fallback: extract from the raw full history that was in the old format
            report.action_history_summary = raw.history.map(h => ({
                t: h.timestamp,
                a: h.action_id,
                p: h.phase
            }));
        }

        return report;
    },

    submitReport: async (explanation) => {
        const reportData = Network._buildSlimReport(explanation);

        try {
            const res = await fetch('api/report_bug', {
                method: 'POST',
                headers: Network.getHeaders(),
                body: JSON.stringify(reportData)
            });
            const data = await res.json();
            return data.success;
        } catch (e) {
            console.error("Report submission failed", e);
            return false;
        }
    }
};
