//! GBA UI language state (mirrors `platforms/3ds/src/lang.rs`).
//!
//! Default is Japanese (engine [`Lang::DEFAULT`]); the Start menu offers a
//! `Language: <autonym>` entry that cycles through the engine's
//! [`Lang::SUPPORTED`] registry, so adding a language needs no change here.
//! The engine resolves bilingual action/choice text through
//! [`PlatformUi::ui_lang`](rabuka_engine::game::platform_ui::PlatformUi::ui_lang),
//! which [`GbaUi`](crate::gba_ui::GbaUi) implements from this state.
//!
//! NOTE: Japanese ability/card source text renders through the baked CJK
//! font tiles; translated UI tables for non-default languages live
//! port-side (same split as the 3DS romfs locales) when they ship.

use rabuka_engine::game::language::Lang;

/// Current UI language. Default: Japanese.
static mut CURRENT_LANG: Lang = Lang::DEFAULT;

pub fn current_lang() -> Lang {
    // Single-threaded GBA: no concurrency on this static.
    unsafe { CURRENT_LANG }
}

pub fn set_lang(lang: Lang) {
    unsafe {
        CURRENT_LANG = lang;
    }
}

/// Cycle to the next supported language and return it.
pub fn cycle_lang() -> Lang {
    let next = current_lang().next();
    set_lang(next);
    next
}
