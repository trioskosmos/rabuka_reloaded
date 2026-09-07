/* tslint:disable */
/* eslint-disable */

export class WasmGameEngine {
    free(): void;
    [Symbol.dispose](): void;
    execute_action(action: any): any;
    get_frame_counter(): bigint;
    get_game_result(): string;
    get_legal_actions(): any;
    get_phase(): string;
    get_state_delta(_since_frame: bigint): any;
    constructor(config: any);
    serialize(): Uint8Array;
}

export function init(): void;

export function wasm_rand_range(max: number): number;

export function wasm_seed(s: number): void;

export function wasm_shuffle(arr: Uint32Array): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmgameengine_free: (a: number, b: number) => void;
    readonly init: () => void;
    readonly wasm_rand_range: (a: number) => number;
    readonly wasm_seed: (a: number) => void;
    readonly wasm_shuffle: (a: number, b: number, c: any) => void;
    readonly wasmgameengine_execute_action: (a: number, b: any) => [number, number, number];
    readonly wasmgameengine_get_frame_counter: (a: number) => bigint;
    readonly wasmgameengine_get_game_result: (a: number) => [number, number];
    readonly wasmgameengine_get_legal_actions: (a: number) => any;
    readonly wasmgameengine_get_phase: (a: number) => [number, number];
    readonly wasmgameengine_get_state_delta: (a: number, b: bigint) => any;
    readonly wasmgameengine_new: (a: any) => [number, number, number];
    readonly wasmgameengine_serialize: (a: number) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
