use wasm_bindgen::prelude::*;

mod game_api;
mod rng_api;

pub use game_api::*;
pub use rng_api::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    #[cfg(feature = "wasm")]
    {
        web_sys::console::log_1(&"Rabuka WASM engine initialized".into());
    }
}