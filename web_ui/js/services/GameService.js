import { State, updateStateData } from '../state.js';
import { log } from '../logger.js';
import { Phase, getAppBaseUrl } from '../constants.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { ModalManager } from '../utils/ModalManager.js';
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

    fetchState: async (networkFacade) => {
        let receivedResponse = false;
        try {
            if (State.replayMode) return;

            if (State.offlineMode) {
                if (!State.wasmAdapter) return;
                const res = await State.wasmAdapter.fetchState();
                if (res.success) {
                    State.lastStateJson = JSON.stringify(res.state);
                    updateStateData(res.state);
                    if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
                }
                return;
            }

            // Simplified: no room/session logic for single-player
            const perfModal = document.getElementById('performance-modal');
            if (perfModal && (perfModal.style.display === 'flex' || perfModal.style.display === 'block')) {
                return;
            }

            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), 2000);

            // Updated endpoint to match current backend
            const url = 'api/game-state';

            const res = await fetch(url, {
                signal: controller.signal
            });
            receivedResponse = true;
            clearTimeout(timeoutId);

            if (!res.ok) {
                const errorBody = await res.text();
                console.error(`[Network] Fetch state failed with status ${res.status}: ${errorBody}`);
                log(`Server error: ${res.status}`, 'error');
                return;
            }

            const raw = await res.text();
            if (raw === State.lastStateJson) return;

            State.lastStateJson = raw;
            // Current backend returns state directly, not wrapped in {success: true, state: ...}
            const data = JSON.parse(raw);

            // Fetch legal actions separately
            let legalActions = [];
            try {
                const actionsRes = await fetch('api/actions');
                if (actionsRes.ok) {
                    const actionsData = await actionsRes.json();
                    legalActions = (actionsData.actions || []).map((action, index) => ({
                        ...action,
                        index: action.index !== undefined ? action.index : index
                    }));
                }
            } catch (e) {
                console.warn('Failed to fetch legal actions:', e);
            }

            // Add legal actions to state
            data.legal_actions = legalActions;
            updateStateData(data);
            State.gameHasStarted = true;

        } catch (e) {
            console.error("Fetch Error:", e);
            if (e.name === 'AbortError') {
                console.warn("[Network] Fetch state timed out.");
            } else {
                console.error("[Network] Critical connection failure.");
                State.resetForNewGame();
                if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
                updateStateData(null);
                log("Connection lost or server unreachable.", 'error');
            }
        }
    },

    sendAction: async (action, networkFacade) => {
        const state = State.data;
        if (!state) return;

        window.pendingAction = true;
        document.body.classList.add('action-pending');
        log(`Action: ${action.action_type}`, 'action');

        try {
            if (State.offlineMode) {
                const res = await State.wasmAdapter.doAction(action);
                if (res.success) {
                    updateStateData(res.state);
                    if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
                    State.lastStateJson = JSON.stringify(res.state);
                    log('Action completed');
                } else {
                    alert(res.error);
                }
                return;
            }

            // Send action in Rust backend format
            const requestBody = {
                action_index: action.index || 0,
                action_type: action.action_type,
                card_id: action.parameters?.card_id,
                card_index: action.parameters?.card_index,
                card_indices: action.parameters?.card_indices,
                card_no: action.parameters?.card_no,
                stage_area: action.parameters?.stage_area,
                use_baton_touch: action.parameters?.use_baton_touch
            };

            const res = await fetch('api/execute-action', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(requestBody)
            });
            const text = await res.text();
            State.lastStateJson = text;
            const data = text ? JSON.parse(text) : null;

            if (!data) {
                throw new Error(`Empty response from api/execute-action (status ${res.status})`);
            }

            // Fetch legal actions after action execution
            let legalActions = [];
            try {
                const actionsRes = await fetch('api/actions');
                if (actionsRes.ok) {
                    const actionsData = await actionsRes.json();
                    legalActions = (actionsData.actions || []).map((action, index) => ({
                        ...action,
                        index: action.index !== undefined ? action.index : index
                    }));
                }
            } catch (e) {
                console.warn('Failed to fetch legal actions:', e);
            }
            data.legal_actions = legalActions;

            updateStateData(data);
            log('Action completed');
        } catch (e) {
            const message = e instanceof Error ? e.message : String(e);
            console.error("[Network] Action request failed:", e);
            alert(message);
        } finally {
            window.pendingAction = false;
            document.body.classList.remove('action-pending');
        }
    },

    resetGame: async (networkFacade) => {
        log('Resetting game...');
        State.resetForNewGame();
        if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();

        if (State.offlineMode) {
            const res = await State.wasmAdapter.resetGame();
            if (res.success) {
                updateStateData(res.state);
                window.lastShownPerformanceHash = "";
                log('New game started');
            }
            return;
        }

        try {
            // Updated endpoint to match current backend
            const res = await fetch('api/init', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' }
            });

            if (!res.ok) {
                log(`Reset failed: ${res.status}`);
                return;
            }

            const text = await res.text();
            State.lastStateJson = text;
            const data = JSON.parse(text);

            // Fetch legal actions after reset
            let legalActions = [];
            try {
                const actionsRes = (await fetch('api/actions').map((action, index) => ({
                        ...action,
                        index: action.index !== undefined ? action.index : index
                    })));
                if (actionsRes.ok) {
                    const actionsData = await actionsRes.json();
                    legalActions = actionsData.actions || [];
                }
            } catch (e) {
                console.warn('Failed to fetch legal actions:', e);
            }
            data.legal_actions = legalActions;

            updateStateData(data);
            window.lastShownPerformanceHash = "";
            log('New game started');
            if (networkFacade?.fetchState) await networkFacade.fetchState();
        } catch (e) {
            log(`Reset error: ${e.message}`);
        }
    },

    startOffline: async (userInitiated = true, networkFacade) => {
        if (userInitiated) {
            const confirmMsg = "Offline mode runs entirely in your browser using WebAssembly.\n\n" +
                "It may take a moment to load the engine.";
            if (!confirm(confirmMsg)) return;
        }

        try {
            if (!State.wasmAdapter) {
                try {
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
            if (networkFacade?.clearPlannerData) networkFacade.clearPlannerData();
            updateStateData(null);

            ModalManager.hide(DOM_IDS.MODAL_ROOM);
            DOMUtils.setText(DOM_IDS.HEADER_DEBUG_INFO, "Offline (WASM)");

            if (networkFacade?.triggerRoomUpdate) networkFacade.triggerRoomUpdate();

            const res = await State.wasmAdapter.resetGame();
            if (res.success) {
                updateStateData(res.state);
                log("Offline Game Started!");
            } else {
                alert("Failed to start offline game: " + res.error);
            }
        } catch (e) {
            console.error(e);
            alert("Offline mode error: " + e.message);
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
