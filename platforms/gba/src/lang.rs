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

/// Pick the port-chrome string for the current language.
///
/// Call sites pass EN first, JA second, so both languages stay visible in
/// review and the baked-font scanner (`tools/font/used_chars.py` reads Rust
/// sources) picks up every JA glyph the UI can show. Non-default languages
/// fall back to English — the same rule as the engine's
/// `display_desc_for` — until their tables ship.
pub fn tr(en: &'static str, ja: &'static str) -> &'static str {
    match current_lang() {
        Lang::Japanese => ja,
        _ => en,
    }
}

/// Main-menu mode labels in display order, current language.
pub fn mode_names() -> [&'static str; 6] {
    [
        tr("VS AI", "VS AI"),
        tr("2 Player", "2人プレイ"),
        tr("Link Host", "リンクホスト"),
        tr("Link Join", "リンク参加"),
        tr("AI vs AI", "AI同士"),
        tr("Deck Builder", "デッキ作成"),
    ]
}

/// Main-menu title (with button hints).
pub fn mode_title() -> &'static str {
    tr("MODE Up/Dn:A/Start", "モード 上下:A/スタート")
}

/// Start/main menu language row for the CURRENT language
/// (mirrors the 3DS `Language: <autonym>` row).
pub fn language_row() -> alloc::string::String {
    alloc::format!("{}: {}", tr("Language", "言語"), current_lang().label())
}
