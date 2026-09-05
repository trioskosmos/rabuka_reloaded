//! Shared UI language selection.
//!
//! The handheld ports used to each own this: the 3DS defined its own `Lang`
//! enum plus a toggle in its START menu / setup flow. That type and the
//! "pick a language" menu are port-agnostic, so they live here now — every
//! console (3DS, GBA, DS, …) shares one `Lang`, and the picker is just the
//! engine's [`crate::game::menu`] selection list with autonym items (each
//! language's own name for itself), so no translation table is needed to ask
//! the question.
//!
//! Translation *tables* stay platform-side (3DS romfs locales, web frontend
//! bundles): baking them into the engine would bloat ROM targets. Ports map
//! [`Lang::code`] (`"en"` / `"jp"`) to their own table files.
//!
//! This module is `no_std`-safe (`core` only) so GBA/DS/PS1 targets can use it.

use crate::game::menu::select_with_initial;
use crate::game::platform_ui::PlatformUi;

/// UI language. Default is Japanese (matches the 3DS default and the web
/// server's `"jp"` default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde_support", derive(serde::Serialize, serde::Deserialize))]
pub enum Lang {
    #[cfg_attr(feature = "serde_support", serde(rename = "en"))]
    English,
    #[cfg_attr(feature = "serde_support", serde(rename = "jp"))]
    Japanese,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::Japanese
    }
}

impl Lang {
    /// The other language (START-menu / R-button toggle behaviour).
    pub fn toggle(self) -> Self {
        match self {
            Lang::English => Lang::Japanese,
            Lang::Japanese => Lang::English,
        }
    }

    /// Display name (autonym): shown in menus without a translation table.
    pub fn label(self) -> &'static str {
        match self {
            Lang::English => "English",
            Lang::Japanese => "日本語",
        }
    }

    /// Short code shared with the web server (`current_lang: "jp" | "en"`)
    /// and the locale file names (`en.json` / `jp.json`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::English => "en",
            Lang::Japanese => "jp",
        }
    }

    /// Parse a [`Lang::code`] (also accepts `"english"` / `"japanese"` /
    /// `"ja"` / `"日本語"`, ASCII case-insensitive).
    pub fn from_code(code: &str) -> Option<Self> {
        if code.eq_ignore_ascii_case("en") || code.eq_ignore_ascii_case("english") {
            Some(Lang::English)
        } else if code.eq_ignore_ascii_case("jp")
            || code.eq_ignore_ascii_case("ja")
            || code.eq_ignore_ascii_case("japanese")
            || code == "日本語"
        {
            Some(Lang::Japanese)
        } else {
            None
        }
    }
}

/// Language picker menu: shows both autonyms (`日本語` / `English`) under a
/// bilingual title, cursor starting on `current`. Returns the picked language.
/// A/Start confirms (same contract as [`crate::game::menu::select`]).
pub fn select_language(ui: &mut dyn PlatformUi, current: Lang) -> Lang {
    log::debug!("[LANG] picker opened (current={:?})", current);
    const ORDER: [Lang; 2] = [Lang::Japanese, Lang::English];
    let items = [Lang::Japanese.label(), Lang::English.label()];
    let initial = ORDER.iter().position(|&l| l == current).unwrap_or(0);
    let picked = ORDER[select_with_initial(ui, &items, "言語 / Language", initial)];
    log::debug!("[LANG] picker picked {:?}", picked);
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_round_trips() {
        assert_eq!(Lang::Japanese.toggle(), Lang::English);
        assert_eq!(Lang::English.toggle(), Lang::Japanese);
    }

    #[test]
    fn labels_and_codes() {
        assert_eq!(Lang::Japanese.label(), "日本語");
        assert_eq!(Lang::English.label(), "English");
        assert_eq!(Lang::Japanese.code(), "jp");
        assert_eq!(Lang::English.code(), "en");
        assert_eq!(Lang::default(), Lang::Japanese);
    }

    #[test]
    fn from_code_accepts_aliases() {
        assert_eq!(Lang::from_code("en"), Some(Lang::English));
        assert_eq!(Lang::from_code("ENGLISH"), Some(Lang::English));
        assert_eq!(Lang::from_code("jp"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("Japanese"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("日本語"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("fr"), None);
    }
}
