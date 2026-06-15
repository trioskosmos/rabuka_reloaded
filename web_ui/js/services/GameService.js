import { State, updateStateData } from '../state.js';
import { log } from '../logger.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS, COLORS } from '../constants_dom.js';

export const GameService = {
    checkSystemStatus: async () => {
        const badge = DOMUtils.getElement(DOM_IDS.SYSTEM_STATUS_BADGE);
        if (!badge) return;
        try {
            const res = await fetch('api/status');
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

    startGameplayPolling: () => {
        if (window._gameplayPollInterval) return;
        let lastVersion = -1;
        window._gameplayPollInterval = setInterval(async () => {
            if (!State.gameHasStarted || !State.roomCode) {
                clearInterval(window._gameplayPollInterval);
                window._gameplayPollInterval = null;
                return;
            }
            try {
                const network = window.Network || null;
                const headers = network?.getHeaders ? network.getHeaders() : {};
                const res = await fetch('api/game-state/version', { headers });
                if (!res.ok) return;
                const data = await res.json();
                if (data.version !== undefined && data.version !== lastVersion) {
                    lastVersion = data.version;
                    await GameService.fetchState(network);
                }
            } catch (_) {}
        }, 500);
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

            const headers = networkFacade?.getHeaders ? networkFacade.getHeaders() : {};
            const res = await fetch('api/game-state', { headers });
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

            if (data.legal_actions) {
                data.legal_actions = data.legal_actions.map((action, index) => ({
                    ...action,
                    index: action.index !== undefined ? action.index : index
                }));
            }

            // If setup modal is still open (e.g., first player getting state via SSE), dismiss it
            const setupModal = document.getElementById(DOM_IDS.MODAL_SETUP);
            if (setupModal && setupModal.style.display !== 'none') {
                const roomModal = document.getElementById(DOM_IDS.MODAL_ROOM);
                if (roomModal) roomModal.style.display = 'none';
                setupModal.style.display = 'none';
            }

            updateStateData(data);
            State.gameHasStarted = true;

            // Start gameplay polling as fallback for unreliable SSE (Cloudflared, proxies)
            GameService.startGameplayPolling();

        } catch (e) {
            console.error("Game state fetch error:", e);
            if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
            updateStateData(null);
        }
    },

    sendAction: async (action, networkFacade) => {
        const state = State.data;
        if (!state) return;

        console.log('DEBUG: Frontend sending action:', action.action_type);

        // For mulligan confirm, inject the locally-selected indices into card_indices
        let extraCardIndices = action.parameters?.card_indices;
        if (action.action_type === 'confirm_mulligan' || action.action_type === 'ConfirmMulligan') {
            extraCardIndices = Array.from(State.localMulliganSelection);
        }

        try {
            // Simple state machine: send action, get new state and actions
            const headers = networkFacade?.getHeaders ? networkFacade.getHeaders() : { 'Content-Type': 'application/json' };
            const res = await fetch('api/execute-action', {
                method: 'POST',
                headers: headers,
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
            console.log('DEBUG: Frontend received new state after action:', data.phase, data.legal_actions?.length || 0, 'actions');
            console.log('DEBUG: Full response data:', JSON.stringify(data, null, 2));
            updateStateData(data);
            log('Action completed');

        } catch (e) {
            console.error("Action error:", e);
            alert(e.message);
        }
    },

    resetGame: async (networkFacade) => {
        log('Resetting game...');
        State.resetForNewGame();
        if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();

        try {
            const headers = networkFacade?.getHeaders ? networkFacade.getHeaders() : { 'Content-Type': 'application/json' };
            const res = await fetch('api/init', {
                method: 'POST',
                headers: headers
            });
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

            if (data.legal_actions) {
                data.legal_actions = data.legal_actions.map((action, index) => ({
                    ...action,
                    index: action.index !== undefined ? action.index : index
                }));
            }

            updateStateData(data);
            window.lastShownPerformanceHash = "";
            State.fetchFrameCounter();
            log('New game started');
            if (networkFacade?.fetchState) await networkFacade.fetchState();
        } catch (e) {
            log(`Reset error: ${e.message}`);
        }
    },

    changeAI: async (aiMode, networkFacade) => {
        try {
            const res = await fetch('api/set_ai', {
                method: 'POST',
                headers: networkFacade?.getHeaders ? networkFacade.getHeaders() : {},
                body: JSON.stringify({ ai_mode: aiMode })
            });
            const data = await res.json();
            if (!data.success) alert('Failed: ' + data.error);
        } catch (e) { console.error(e); }
    }
};
