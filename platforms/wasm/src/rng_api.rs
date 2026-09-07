use wasm_bindgen::prelude::*;
use rabuka_engine::rng::{seed, shuffle_slice};

#[wasm_bindgen]
pub fn wasm_seed(s: u32) {
    seed(s);
}

#[wasm_bindgen]
pub fn wasm_shuffle(arr: &mut [u32]) {
    shuffle_slice(arr);
}

#[wasm_bindgen]
pub fn wasm_rand_range(max: usize) -> usize {
    rabuka_engine::rng::rand_range(max)
}