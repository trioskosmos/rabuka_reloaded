import { State, updateStateData } from '../state.js';
import { log } from '../logger.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS, COLORS } from '../constants_dom.js';
import { apiFetch, Network } from '../network.js';

/** Backend responses carry legal_actions as {action_type, description, parameters}.
 *  Ensure each has a stable numeric `index` for the UI. */
function normalizeLegalActions(data) {
    if (data && Array.isArray(data.legal_actions)) {
        data.legal_actions = data.legal_actions.map((action, index) => ({
            ...action,
            index: action.index !== undefined ? action.index : index
        }));
    }
    return data;
}

export const GameService = {
    checkSystemStatus: async () => {
        const badge = DOMUtils.getElement(DOM_IDS.SYSTEM_STATUS_BADGE);
        if (!badge) return;
        try {
            const res = await apiFetch('api/status');
            const data = await res.json();
            if (data.status === 'rust_server') {
                const cardCount = (data.members || 0) + (data.lives || 0);
                DOMUtils.setText(DOM_IDS.SYSTEM_STATUS_BADGE, cardCount > 0 ? `ONLINE: ${cardCount} Cards` : "ONLINE: 0 Cards (ERROR)");
                DOMUtils.setBackground(DOM_IDS.SYSTEM_STATUS_BADGE, cardCount > 100 ? COLORS.ONLINE : COLORS.WARNING);
                badge.title = `Members: ${data.members}, Lives: ${data.lives} | ID: ${data.instance_id}`;

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
                DOMUtils.setText(DOM_IDS.SYSTEM_STATUS_BADGE, "UNKNOWN");
                DOMUtils.setBackground(DOM_IDS.SYSTEM_STATUS_BADGE, COLORS.UNKNOWN);
            }
        } catch (e) {
            DOMUtils.setText(DOM_IDS.SYSTEM_STATUS_BADGE, "OFFLINE");
            DOMUtils.setBackground(DOM_IDS.SYSTEM_STATUS_BADGE, COLORS.OFFLINE);
        }
        return null;
    },

    _lastKnownVersion: -1,
    _sseConnected: false,
    // Coalescing flags: every apiFetch is a real request now, so overlapping
    // callers (250ms poll + SSE push + sendAction + AiDriver refresh) must
    // collapse instead of piling up over a slow tunnel.
    _fetchInFlight: false,
    _fetchQueued: false,
    _versionCheckInFlight: false,

    setSseConnected: (connected) => {
        GameService._sseConnected = connected;
        if (connected) {
            GameService.stopGameplayPolling();
        }
    },

    startGameplayPolling: () => {
        // Skip polling entirely when SSE is active - SSE pushes frame_ids
        if (GameService._sseConnected) return;
        if (window._gameplayPollInterval) return;
        window._gameplayPollInterval = setInterval(async () => {
            if (!State.gameHasStarted || !State.roomCode) {
                clearInterval(window._gameplayPollInterval);
                window._gameplayPollInterval = null;
                return;
            }
            await GameService.checkVersionAndFetch();
        }, 250);
    },

    checkVersionAndFetch: async () => {
        // Skip version check when SSE is active - SSE message already has frame_id
        if (GameService._sseConnected) return;
        // Interval ticks can overlap a slow response — collapse, don't pile up.
        if (GameService._versionCheckInFlight) return;
        GameService._versionCheckInFlight = true;
        try {
            const res = await apiFetch('api/game-state/version');
            if (!res.ok) return;
            const data = await res.json();
            console.log('[GameService] Version check:', data.version, 'lastKnown:', GameService._lastKnownVersion);
            if (data.version !== undefined && data.version !== GameService._lastKnownVersion) {
                GameService._lastKnownVersion = data.version;
                console.log('[GameService] Version changed, fetching state');
                // Fetch full state immediately — the delta endpoint omits
                // board zones and legal_actions, so delta-only sync leaves
                // the UI stale and forces a 2s fallback fetch anyway.
                await GameService.fetchState(Network);
            }
        } catch (e) {
            console.error('[GameService] Version check error:', e);
        } finally {
            GameService._versionCheckInFlight = false;
        }
    },

    triggerVersionCheck: (frameId) => {
        if (frameId !== undefined) {
            // SSE push means state is ready — fetch it directly instead of
            // spinning on the delta endpoint.
            GameService._lastKnownVersion = frameId;
            GameService.fetchState(Network);
        } else {
            GameService.checkVersionAndFetch();
        }
    },

    stopGameplayPolling: () => {
        if (window._gameplayPollInterval) {
            clearInterval(window._gameplayPollInterval);
            window._gameplayPollInterval = null;
        }
    },

    fetchState: async (networkFacade) => {
        // Collapse overlapping callers into a single request + one queued
        // follow-up, so the newest state is always picked up.
        if (GameService._fetchInFlight) {
            GameService._fetchQueued = true;
            return;
        }
        GameService._fetchInFlight = true;
        try {
            do {
                GameService._fetchQueued = false;
                if (State.replayMode) return;

                const res = await apiFetch('api/game-state');
                if (!res.ok) {
                    throw new Error(`State fetch failed: ${res.status}`);
                }

                const data = await res.json();

                // Room was closed (opponent left) — redirect to lobby
                if (data.room_closed) {
                    if (window.handleRoomClosed) {
                        window.handleRoomClosed();
                    }
                    break;
                }

                // Room not ready yet (opponent hasn't submitted deck) — keep current state
                if (data.room_not_ready) {
                    break;
                }

                normalizeLegalActions(data);

                // If setup modal is still open (e.g., first player getting state via SSE), dismiss it
                const setupModal = document.getElementById(DOM_IDS.MODAL_SETUP);
                if (setupModal && setupModal.style.display !== 'none') {
                    const roomModal = document.getElementById(DOM_IDS.MODAL_ROOM);
                    if (roomModal) roomModal.style.display = 'none';
                    setupModal.style.display = 'none';
                }

                if (data.frame_counter !== undefined) {
                    GameService._lastKnownVersion = data.frame_counter;
                    State._frameCounter = data.frame_counter;
                }
                updateStateData(data);
                State.gameHasStarted = true;
                GameService.startGameplayPolling();
            } while (GameService._fetchQueued);
        } catch (e) {
            console.error("Game state fetch error:", e);
            if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
            // Keep the current board on transient errors — wiping the state
            // here blanked the UI on every hiccup.
        } finally {
            GameService._fetchInFlight = false;
            // A fetch requested while we were busy: run one follow-up so the
            // newest state is always picked up.
            if (GameService._fetchQueued) {
                GameService._fetchQueued = false;
                GameService.fetchState(networkFacade);
            }
        }
    },

    sendAction: async (action, networkFacade) => {
        console.log('[GameService] sendAction:', action);
        const state = State.data;
        if (!state) return;

        // For mulligan/live card confirm, inject the locally-selected indices into card_indices
        let extraCardIndices = action.parameters?.card_indices;
        if (action.action_type === 'confirm_mulligan' || action.action_type === 'ConfirmMulligan') {
            extraCardIndices = Array.from(State.localMulliganSelection);
        }
        if (action.action_type === 'confirm_live_card_set' || action.action_type === 'ConfirmLiveCardSet') {
            extraCardIndices = Array.from(State.localLiveCardSelection);
        }

        // Optimistic local prediction — apply deterministic state changes immediately
        // so the UI feels instant even over the tunnel. Server response overwrites.
        let predicted = false;
        const actionType = action.action_type;
        const playerKey = State.perspectivePlayer === 0 ? 'player1' : 'player2';
        const player = state[playerKey];

        if (actionType === 'play_member_to_stage' && player?.hand?.cards && action.parameters?.card_index !== undefined && action.parameters?.stage_area) {
            const cardIndex = action.parameters.card_index;
            const stageArea = action.parameters.stage_area;
            const card = player.hand.cards[cardIndex];
            if (card && player.stage?.[stageArea] !== undefined) {
                const predictedState = JSON.parse(JSON.stringify(state));
                const p = predictedState[playerKey];
                const removed = p.hand.cards.splice(cardIndex, 1);
                p.hand.count = p.hand.cards.length;

                if (!p.stage[stageArea]) {
                    p.stage[stageArea] = [];
                }
                if (Array.isArray(p.stage[stageArea])) {
                    p.stage[stageArea].push(removed[0]);
                } else {
                    p.stage[stageArea] = [removed[0]];
                }
                if (predictedState.state_id !== undefined) {
                    predictedState._predictedStateId = predictedState.state_id;
                    predictedState.state_id = `predicted-${predictedState.state_id}`;
                }
                updateStateData(predictedState);
                predicted = true;
            }
        }

        const actionStart = performance.now();
        try {
            const res = await apiFetch('api/execute-action', {
                method: 'POST',
                body: JSON.stringify({
                    action_index: action.index || 0,
                    action_type: action.action_type,
                    card_id: action.parameters?.card_id,
                    card_index: action.parameters?.card_index,
                    card_indices: extraCardIndices,
                    card_no: action.parameters?.card_no,
                    stage_area: action.parameters?.stage_area,
                    use_baton_touch: action.parameters?.use_baton_touch
                })
            });

            if (!res.ok) {
                const errorText = await res.text();
                let message = errorText;
                try {
                    const parsed = JSON.parse(errorText);
                    message = parsed.error || parsed.message || errorText;
                } catch { /* keep raw text */ }
                // Turn races (double-click, AI reply landing first) come back
                // as 403s — flag them so the catch below resyncs silently.
                const err = new Error(`Action failed: ${message}`);
                err.status = res.status;
                err.isTurnRace = res.status === 403
                    || /not your turn|waiting for opponent/i.test(message);
                throw err;
            }

            const data = await res.json();

            // Handle minimal ActionResult (2-human PVP) vs full
            // GameStateResponse (sandbox/PVE/VS AI — server settles AI
            // replies inline, so the state is already complete).
            if (data.success !== undefined) {
                // ActionResult format (PVP) - success + frame_id, no full state
                console.log('[GameService] ActionResult:', data);
                State._frameCounter = data.frame_id;
                State._actionLatency = Math.round(performance.now() - actionStart);

                GameService._lastKnownVersion = data.frame_id;
                await GameService.fetchState(networkFacade);
            } else if (data.frame_counter !== undefined) {
                // Full state response (sandbox/PVE, and VS AI rooms now too)
                State._frameCounter = data.frame_counter;
                GameService._lastKnownVersion = data.frame_counter;
                State._actionLatency = Math.round(performance.now() - actionStart);
                normalizeLegalActions(data);
                updateStateData(data);
            } else {
                // Fallback - assume full state
                State._actionLatency = Math.round(performance.now() - actionStart);
                normalizeLegalActions(data);
                if (data.frame_counter !== undefined) {
                    State._frameCounter = data.frame_counter;
                    GameService._lastKnownVersion = data.frame_counter;
                }
                updateStateData(data);
            }
            log('Action completed');

        } catch (e) {
            console.error("Action error:", e);
            // Turn races are expected (double-click, AI reply landing first):
            // resync silently instead of popping an alert.
            if (e.isTurnRace) {
                if (networkFacade) await GameService.fetchState(networkFacade);
                return;
            }
            // Revert predicted state by re-fetching authoritative state
            if (predicted && networkFacade) {
                await GameService.fetchState(networkFacade);
            }
            alert(e.message);
        }
    },

    resetGame: async (networkFacade) => {
        log('Resetting game...');
        State.resetForNewGame();
        if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();

        try {
            const res = await apiFetch('api/init', { method: 'POST' });
            const text = await res.text();
            if (!res.ok) {
                let message = `Reset failed (${res.status})`;
                if (text) {
                    try {
                        const errorData = JSON.parse(text);
                        message = errorData.error || errorData.message || message;
                    } catch {
                        message = text;
                    }
                }
                log(message, 'error');
                return;
            }
            State.lastStateJson = text;
            const data = JSON.parse(text);

            normalizeLegalActions(data);

            updateStateData(data);
            window.lastShownPerformanceHash = "";
            if (data.frame_counter !== undefined) State._frameCounter = data.frame_counter;
            log('New game started');
            if (networkFacade?.fetchState) await networkFacade.fetchState();
        } catch (e) {
            log(`Reset error: ${e.message}`);
        }
    },
};
