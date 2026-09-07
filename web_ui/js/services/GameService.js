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

    startGameplayPolling: () => {
        if (window._gameplayPollInterval) return;
        window._gameplayPollInterval = setInterval(async () => {
            if (!State.gameHasStarted || !State.roomCode) {
                clearInterval(window._gameplayPollInterval);
                window._gameplayPollInterval = null;
                return;
            }
            await GameService.checkVersionAndFetch();
        }, 500);
    },

    checkVersionAndFetch: async () => {
        try {
            const res = await apiFetch('api/game-state/version');
            if (!res.ok) return;
            const data = await res.json();
            console.log('[GameService] Version check:', data.version, 'lastKnown:', GameService._lastKnownVersion);
            if (data.version !== undefined && data.version !== GameService._lastKnownVersion) {
                GameService._lastKnownVersion = data.version;
                console.log('[GameService] Version changed, polling delta');
                // Use delta instead of full state fetch
                await GameService.pollDelta(data.version, Network);
            }
        } catch (e) {
            console.error('[GameService] Version check error:', e);
        }
    },

    triggerVersionCheck: (frameId) => {
        if (frameId !== undefined) {
            GameService._lastKnownVersion = frameId;
            GameService.pollDelta(frameId, Network);
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
        try {
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
                return;
            }

            // Room not ready yet (opponent hasn't submitted deck) — keep current state
            if (data.room_not_ready) {
                return;
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

        } catch (e) {
            console.error("Game state fetch error:", e);
            if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
            updateStateData(null);
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
                throw new Error(`Action failed: ${errorText}`);
            }

            const data = await res.json();
            
            // Handle new minimal ActionResult (PVP) vs full GameStateResponse (sandbox/PVE)
            if (data.success !== undefined) {
                // ActionResult format (PVP) - success + frame_id, no full state
                console.log('[GameService] ActionResult:', data);
                State._frameCounter = data.frame_id;
                State._actionLatency = Math.round(performance.now() - actionStart);
                
                if (data.state_delta) {
                    // Apply delta if provided
                    applyStateDelta(data.state_delta);
                } else {
                    // Poll delta endpoint or wait for SSE
                    await pollDelta(data.frame_id, networkFacade);
                }
            } else if (data.frame_counter !== undefined) {
                // Legacy full state response (sandbox/PVE)
                State._frameCounter = data.frame_counter;
                State._actionLatency = Math.round(performance.now() - actionStart);
                updateStateData(data);
            } else {
                // Fallback - assume full state
                State._actionLatency = Math.round(performance.now() - actionStart);
                updateStateData(data);
            }
            log('Action completed');

        } catch (e) {
            console.error("Action error:", e);
            // Revert predicted state by re-fetching authoritative state
            if (predicted && networkFacade) {
                await GameService.fetchState(networkFacade);
            }
alert(e.message);
        }
    },

    // Poll delta endpoint until we get changes since frame_id
    pollDelta: async (frameId, networkFacade) => {
        const maxAttempts = 20;
        for (let i = 0; i < maxAttempts; i++) {
            try {
                const res = await apiFetch(`api/game-state/delta?since=${frameId}`);
                if (res.ok) {
                    const delta = await res.json();
                    if (!delta.no_changes) {
                        console.log('[GameService] Delta received:', delta);
                        GameService.applyStateDelta(delta);
                        return;
                    }
                }
            } catch (e) {
                console.warn('[GameService] Delta poll failed:', e);
            }
            await new Promise(r => setTimeout(r, 100));
        }
        console.log('[GameService] Delta timeout, fetching full state');
        if (networkFacade?.fetchState) await networkFacade.fetchState();
    },

    // Apply delta to current state
    applyStateDelta: (delta) => {
        const state = State.data;
        if (!state) return;
        
        const newState = JSON.parse(JSON.stringify(state));
        
        if (delta.phase) newState.phase = delta.phase;
        if (delta.active_player !== undefined) newState.active_player = delta.active_player;
        if (delta.rps_winner !== undefined) newState.rps_winner = delta.rps_winner;
        if (delta.player1_rps_choice !== undefined) newState.player1_rps_choice = delta.player1_rps_choice;
        if (delta.player2_rps_choice !== undefined) newState.player2_rps_choice = delta.player2_rps_choice;
        if (delta.pending_choice) newState.pending_choice = delta.pending_choice;
        if (delta.legal_actions) newState.legal_actions = delta.legal_actions;
        if (delta.player1) newState.player1 = delta.player1;
        if (delta.player2) newState.player2 = delta.player2;
        
        if (delta.log_entries && delta.log_entries.length > 0) {
            newState.log = (newState.log || []).concat(delta.log_entries);
        }
        
        if (delta.zone_changes && delta.zone_changes.length > 0) {
            for (const zc of delta.zone_changes) {
                console.log('[GameService] Zone change:', zc);
            }
        }
        
        if (delta.frame_id !== undefined) {
            State._frameCounter = delta.frame_id;
        }
        
        updateStateData(newState);
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

    changeAI: async (aiMode, networkFacade) => {
        try {
            const res = await apiFetch('api/set_ai', {
                method: 'POST',
                body: JSON.stringify({ ai_mode: aiMode })
            });
            const data = await res.json();
            if (!data.success) alert('Failed: ' + data.error);
        } catch (e) { console.error(e); }
    }
};
