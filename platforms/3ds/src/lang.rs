// Current UI language state + translation shortcuts.

use crate::i18n;
use crate::i18n::Lang;

/// Current UI language. Default: Japanese. Toggled via START menu.
#[cfg(feature = "3ds")]
static mut CURRENT_LANG: Lang = Lang::Japanese;

#[cfg(feature = "3ds")]
pub fn current_lang() -> Lang {
    unsafe { CURRENT_LANG }
}

#[cfg(feature = "3ds")]
pub fn set_lang(lang: Lang) {
    unsafe {
        CURRENT_LANG = lang;
    }
}

/// Shorthand for translating a key in the current language.
#[cfg(feature = "3ds")]
pub fn tl(key: &str) -> String {
    i18n::t(key, current_lang())
}

/// Shorthand for formatting a translated key with params.
#[cfg(feature = "3ds")]
pub fn tl_fmt(key: &str, params: &[(&str, &str)]) -> String {
    i18n::t_fmt(key, current_lang(), params)
}
