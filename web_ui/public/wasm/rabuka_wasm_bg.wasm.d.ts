/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const __wbg_wasmgameengine_free: (a: number, b: number) => void;
export const init: () => void;
export const wasm_rand_range: (a: number) => number;
export const wasm_seed: (a: number) => void;
export const wasm_shuffle: (a: number, b: number, c: any) => void;
export const wasmgameengine_execute_action: (a: number, b: any) => [number, number, number];
export const wasmgameengine_get_frame_counter: (a: number) => bigint;
export const wasmgameengine_get_game_result: (a: number) => [number, number];
export const wasmgameengine_get_legal_actions: (a: number) => any;
export const wasmgameengine_get_phase: (a: number) => [number, number];
export const wasmgameengine_get_state_delta: (a: number, b: bigint) => any;
export const wasmgameengine_new: (a: any) => [number, number, number];
export const wasmgameengine_serialize: (a: number) => [number, number];
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_exn_store: (a: number) => void;
export const __externref_table_alloc: () => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_start: () => void;
