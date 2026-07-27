use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    English,
    Japanese,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Lang::English => Lang::Japanese,
            Lang::Japanese => Lang::English,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::English => "English",
            Lang::Japanese => "日本語",
        }
    }
}

struct Translations {
    en: HashMap<String, String>,
    jp: HashMap<String, String>,
    names: HashMap<String, String>,
    ability_en: HashMap<String, String>,
}

static I18N: OnceLock<Translations> = OnceLock::new();

fn load_json(path: &str) -> HashMap<String, String> {
    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) => {
            #[cfg(feature = "3ds")]
            unsafe {
                let msg = format!("i18n: failed to load {}: {}\0", path, e);
                extern "C" {
                    fn _3ds_debug_print(s: *const u8);
                }
                _3ds_debug_print(msg.as_ptr());
            }
            let _ = e;
            return HashMap::new();
        }
    };
    match serde_json::from_str::<serde_json::Value>(&data) {
        Ok(serde_json::Value::Object(map)) => map
            .into_iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
            .collect(),
        _ => HashMap::new(),
    }
}

pub fn init() {
    let translations = Translations {
        en: load_json("romfs:/locales/en.json"),
        jp: load_json("romfs:/locales/jp.json"),
        names: load_json("romfs:/locales/names.json"),
        ability_en: load_json("romfs:/locales/ability_en.json"),
    };
    // OnceLock only sets once; subsequent calls are no-ops.
    // For language toggle, we don't need to re-init — just swap CURRENT_LANG.
    let _ = I18N.set(translations);
}

/// Translate a UI string key. Returns the value for the given language,
/// falling back to the key itself.
pub fn t(key: &str, lang: Lang) -> String {
    let Some(i18n) = I18N.get() else {
        return key.to_string();
    };
    let map = match lang {
        Lang::English => &i18n.en,
        Lang::Japanese => &i18n.jp,
    };
    map.get(key).cloned().unwrap_or_else(|| key.to_string())
}

/// Translate a UI string with simple {param} substitution.
pub fn t_fmt(key: &str, lang: Lang, params: &[(&str, &str)]) -> String {
    let mut s = t(key, lang);
    for (placeholder, value) in params {
        s = s.replace(&format!("{{{}}}", placeholder), value);
    }
    s
}

/// Translate a Japanese card name to English.
/// Returns None if no translation exists or lang is Japanese.
pub fn translate_card_name(jp_name: &str, lang: Lang) -> Option<String> {
    if lang == Lang::Japanese {
        return None;
    }
    let i18n = I18N.get()?;
    i18n.names.get(jp_name).cloned()
}

/// Get the display name for a card in the given language.
pub fn card_display_name(jp_name: &str, lang: Lang) -> String {
    match lang {
        Lang::Japanese => jp_name.to_string(),
        Lang::English => {
            translate_card_name(jp_name, Lang::English).unwrap_or_else(|| jp_name.to_string())
        }
    }
}

/// Translate ability text. In English mode, looks up the full Japanese text
/// in the pre-translated table. Falls back to original (Japanese) text.
pub fn translate_ability(full_text: &str, lang: Lang) -> String {
    if lang == Lang::Japanese {
        return full_text.to_string();
    }
    let Some(i18n) = I18N.get() else {
        return full_text.to_string();
    };
    i18n.ability_en
        .get(full_text)
        .cloned()
        .unwrap_or_else(|| full_text.to_string())
}
