// Web Worker wrapper for Rabuka WASM engine
// Runs the game engine off the main thread using wasm-bindgen generated module

// NOTE: this file lives at web_ui/src/workers/gameWorker.js; the
// wasm-bindgen bundle is checked in at web_ui/public/wasm/. Both the local
// backend (engine web_server mounts /wasm -> ../web_ui/public/wasm) and
// GitHub Pages (docs/wasm/) serve it at /wasm/.
import * as wasmModule from '../../public/wasm/rabuka_wasm.js';

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
                // wasm_shuffle takes &mut [u32]: the glue expects a Uint32Array
                // (copy-back into a plain Array would throw). Normalize here so
                // callers can pass plain arrays.
                self.postMessage({ type: 'SHUFFLE_DONE', payload: Array.from(shuffleInPlace(payload)) });
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
    // Initialize wasm-bindgen module. Default init resolves
    // rabuka_wasm_bg.wasm relative to rabuka_wasm.js (import.meta.url), so
    // this works wherever the bundle is served (/public/wasm locally,
    // /public/wasm + /wasm on Pages) without hardcoding a path.
    await wasmModule.default();
    
    // Create game engine
    engine = new wasmModule.WasmGameEngine(config);
    
    self.postMessage({ type: 'INIT_DONE', payload: null });
}

function executeAction(payload) {
    const { id, ...action } = payload;
    try {
        const result = normalizeBigints(engine.execute_action(action));
        return { id, ...result };
    } catch (e) {
        return { id, success: false, error: e.message, frame_id: Number(engine.get_frame_counter()) };
    }
}

function getStateDelta(sinceFrame) {
    // sinceFrame arrives as a JS number over postMessage; the binding takes u64 (bigint).
    const since = typeof sinceFrame === 'bigint' ? sinceFrame : BigInt(sinceFrame ?? 0);
    return normalizeBigints(engine.get_state_delta(since));
}

function getLegalActions() {
    return normalizeBigints(engine.get_legal_actions());
}

// serde-wasm-bindgen maps Rust u64 (frame_id) to JS bigint, but the UI state
// layer (State._frameCounter, comparisons, JSON) expects numbers. Frame ids
// are tiny, so a lossless Number() conversion is safe and keeps structured
// clone + downstream arithmetic working.
function normalizeBigints(value) {
    if (typeof value === 'bigint') return Number(value);
    if (Array.isArray(value)) return value.map(normalizeBigints);
    if (value && typeof value === 'object') {
        const out = {};
        for (const [k, v] of Object.entries(value)) out[k] = normalizeBigints(v);
        return out;
    }
    return value;
}

function shuffleInPlace(payload) {
    const arr = payload instanceof Uint32Array ? payload : Uint32Array.from(payload ?? []);
    wasmModule.wasm_shuffle(arr);
    return arr;
}

function serialize() {
    return Array.from(engine.serialize());
}