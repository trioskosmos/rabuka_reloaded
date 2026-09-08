// WASM-enabled GameService - replaces server API calls with local Web Worker
import { State, updateStateData } from '../state.js';
import { log } from '../logger.js';
import { DOMUtils } from '../utils/DOMUtils.js';
import { DOM_IDS, COLORS } from '../constants_dom.js';

let gameWorker = null;
let actionId = 0;
const pendingActions = new Map();

export const WasmGameService = {
    // Initialize WASM engine in Web Worker
    init: async (config) => {
        return new Promise((resolve, reject) => {
            // NOTE: this file lives at web_ui/js/services/; the worker source
            // is at web_ui/src/workers/gameWorker.js. Served paths: local
            // backend mounts ../web_ui at / (so /src/workers/gameWorker.js),
            // GitHub Pages serves docs/ at root (deploy copies src/workers).
            gameWorker = new Worker(new URL('../../src/workers/gameWorker.js', import.meta.url), { type: 'module' });
            
            gameWorker.onmessage = (event) => {
                const { type, payload } = event.data;
                WasmGameService.handleWorkerMessage(type, payload);
            };
            
            gameWorker.onerror = (error) => {
                console.error('Worker error:', error);
                reject(error);
            };
            
            // Wait for INIT_DONE
            const initHandler = (event) => {
                if (event.data.type === 'INIT_DONE') {
                    gameWorker.removeEventListener('message', initHandler);
                    resolve();
                } else if (event.data.type === 'ERROR') {
                    gameWorker.removeEventListener('message', initHandler);
                    reject(new Error(event.data.payload));
                }
            };
            gameWorker.addEventListener('message', initHandler);
            
            // Send init config
            gameWorker.postMessage({ 
                type: 'INIT', 
                payload: config 
            });
        });
    },

    // Handle messages from worker
    handleWorkerMessage: (type, payload) => {
        switch (type) {
            case 'ACTION_RESULT':
                WasmGameService.handleActionResult(payload);
                break;
            case 'STATE_DELTA':
                WasmGameService.applyStateDelta(payload);
                break;
            case 'LEGAL_ACTIONS':
                WasmGameService.handleLegalActions(payload);
                break;
            case 'EXPORT_DATA':
                WasmGameService.handleExport(payload);
                break;
            case 'ERROR':
                console.error('Worker error:', payload);
                WasmGameService.handleActionError(payload);
                break;
        }
    },

    // Send action to worker
    sendAction: async (action) => {
        const state = State.data;
        if (!state) return;

        // Apply optimistic update immediately for instant UI
        WasmGameService.applyOptimisticUpdate(action);

        const id = ++actionId;
        return new Promise((resolve, reject) => {
            pendingActions.set(id, { resolve, reject, action });
            
            gameWorker.postMessage({
                type: 'ACTION',
                payload: {
                    id,
                    ...WasmGameService.normalizeAction(action)
                }
            });
            
            // Timeout fallback
            setTimeout(() => {
                if (pendingActions.has(id)) {
                    pendingActions.delete(id);
                    // Revert optimistic update by fetching authoritative state
                    WasmGameService.fetchState();
                    reject(new Error('Action timeout'));
                }
            }, 5000);
        });
    },

    normalizeAction: (action) => {
        const params = action.parameters || {};
        return {
            action_type: action.action_type,
            card_id: params.card_id,
            card_indices: params.card_indices,
            stage_area: params.stage_area,
            use_baton_touch: params.use_baton_touch,
            ability_index: params.ability_index
        };
    },

    handleActionResult: (payload) => {
        const { id, success, error, frame_id, state_delta } = payload;
        const pending = pendingActions.get(id);
        
        if (pending) {
            pendingActions.delete(id);
            if (success) {
                State._frameCounter = frame_id;
                if (state_delta) {
                    WasmGameService.applyStateDelta(state_delta);
                }
                pending.resolve(payload);
            } else {
                // Revert optimistic update
                WasmGameService.fetchState();
                pending.reject(new Error(error));
            }
        }
    },

    handleActionError: (error) => {
        // Find and reject pending action
        for (const [id, pending] of pendingActions) {
            pendingActions.delete(id);
            WasmGameService.fetchState(); // Revert optimistic
            pending.reject(new Error(error));
            break;
        }
    },

    applyOptimisticUpdate: (action) => {
        // Same optimistic logic as before for instant UI feedback
        const state = State.data;
        if (!state) return;

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
                updateStateData(predictedState);
            }
        }
    },

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
        if (delta.legal_actions) {
            delta.legal_actions.forEach((action, index) => {
                action.index = action.index !== undefined ? action.index : index;
            });
            newState.legal_actions = delta.legal_actions;
        }

        if (delta.frame_id !== undefined) {
            State._frameCounter = delta.frame_id;
        }

        updateStateData(newState);
    },

    handleLegalActions: (actions) => {
        const state = State.data;
        if (!state) return;

        const newState = JSON.parse(JSON.stringify(state));
        actions.forEach((action, index) => {
            action.index = action.index !== undefined ? action.index : index;
        });
        newState.legal_actions = actions;
        updateStateData(newState);
    },

    // Last exported snapshot (Uint8Array-compatible array). exportGame()
    // resolves via its own message listener; this handler just caches so the
    // shared onmessage dispatch never throws on EXPORT_DATA.
    lastExport: null,
    handleExport: (payload) => {
        WasmGameService.lastExport = payload;
    },

    // Fetch full state from worker
    fetchState: () => {
        gameWorker.postMessage({ type: 'GET_DELTA', payload: 0 });
    },

    // Get legal actions
    getLegalActions: () => {
        gameWorker.postMessage({ type: 'GET_ACTIONS', payload: null });
    },

    // Export game state for replay
    exportGame: () => {
        return new Promise((resolve) => {
            const handler = (event) => {
                if (event.data.type === 'EXPORT_DATA') {
                    gameWorker.removeEventListener('message', handler);
                    resolve(event.data.payload);
                }
            };
            gameWorker.addEventListener('message', handler);
            gameWorker.postMessage({ type: 'EXPORT', payload: null });
        });
    },

    // Seed RNG for commit-reveal
    seedRng: (seed) => {
        gameWorker.postMessage({ type: 'SEED', payload: seed });
    },

    // Shuffle array using deterministic RNG
    shuffleArray: (arr) => {
        gameWorker.postMessage({ type: 'SHUFFLE', payload: arr });
    },

    // Cleanup
    destroy: () => {
        if (gameWorker) {
            gameWorker.terminate();
            gameWorker = null;
        }
        pendingActions.clear();
    }
};

// Backwards compatibility - can swap GameService with WasmGameService
export const GameService = WasmGameService;