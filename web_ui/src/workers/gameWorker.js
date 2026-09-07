// Web Worker wrapper for Rabuka WASM engine
// Runs the game engine off the main thread using wasm-bindgen generated module

import * as wasmModule from '../wasm/rabuka_wasm.js';

let engine = null;

self.onmessage = async (event) => {
    const { type, payload } = event.data;
    
    try {
        switch (type) {
            case 'INIT':
                await initWasm(payload);
                break;
            case 'ACTION':
                const result = executeAction(payload);
                self.postMessage({ type: 'ACTION_RESULT', payload: result });
                break;
            case 'GET_DELTA':
                const delta = getStateDelta(payload);
                self.postMessage({ type: 'STATE_DELTA', payload: delta });
                break;
            case 'GET_ACTIONS':
                const actions = getLegalActions();
                self.postMessage({ type: 'LEGAL_ACTIONS', payload: actions });
                break;
            case 'EXPORT':
                const exportData = serialize();
                self.postMessage({ type: 'EXPORT_DATA', payload: exportData });
                break;
            case 'SEED':
                wasmModule.wasm_seed(payload);
                self.postMessage({ type: 'SEED_DONE', payload: null });
                break;
            case 'SHUFFLE':
                wasmModule.wasm_shuffle(payload);
                self.postMessage({ type: 'SHUFFLE_DONE', payload: null });
                break;
            default:
                console.warn('Unknown message type:', type);
        }
    } catch (error) {
        console.error('Worker error:', error);
        self.postMessage({ type: 'ERROR', payload: error.message });
    }
};

async function initWasm(config) {
    // Initialize wasm-bindgen module
    await wasmModule.default({ module_or_path: '/wasm/rabuka_wasm_bg.wasm' });
    
    // Create game engine
    engine = new wasmModule.WasmGameEngine(config);
    
    self.postMessage({ type: 'INIT_DONE', payload: null });
}

function executeAction(payload) {
    const { id, ...action } = payload;
    try {
        const result = engine.execute_action(action);
        return { id, ...result };
    } catch (e) {
        return { id, success: false, error: e.message, frame_id: engine.get_frame_counter() };
    }
}

function getStateDelta(sinceFrame) {
    return engine.get_state_delta(BigInt(sinceFrame));
}

function getLegalActions() {
    return engine.get_legal_actions();
}

function serialize() {
    return Array.from(engine.serialize());
}