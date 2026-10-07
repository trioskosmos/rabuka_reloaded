//! Choice-prompt translation: the canonical English template table and the
//! parameterised EN→JA translator.
//!
//! `CHOICE_PROMPT_TEMPLATES_EN` is consumed by the startup self-check
//! (`i18n_self_check` in `main.rs`) to assert that every generic instruction
//! has a Japanese translation. `translate_choice_prompt_en_to_ja` handles the
//! parameterised templates ("Select up to N card(s) to keep") that an exact
//! match table cannot, since the number varies.


use super::*;

// ── Japanese description ──────────────────────────────────────────────

// ── Japanese description helpers (delegates to locale-parameterized inner) ──

pub fn zone_label_ja(zone: Option<&str>) -> &str {
    zone_label_inner(zone, true)
}

pub fn state_verb_ja(state: Option<&str>) -> &str {
    state_verb_inner(state, true)
}

pub fn resource_label_ja(r: Option<&str>) -> &str {
    resource_label_inner(r, true)
}

/// Canonical English choice-prompt templates. Used by the startup self-check
/// (`i18n_self_check` in main.rs) to assert that every generic instruction has a
/// Japanese translation. Keep this in sync with the templates handled below.
pub const CHOICE_PROMPT_TEMPLATES_EN: &[&str] = &[
    "Select up to 3 card(s) to keep",
    "Select up to 3 more card(s) from hand to keep",
    "Select up to 1 member(s) to change state",
    "Select up to 3 card(s) from the 5 looked-at cards (or skip)",
    "Select up to 2 more card(s) from the 4 remaining looked-at cards",
    "Repeat effect?",
    "Pay optional cost or skip",
];

/// Single source of truth for translating a generic English choice *instruction*
/// prompt into Japanese. Handles the parameterized templates (e.g. "Select up to
/// N card(s) to keep") that an exact-match table cannot, since the number varies.
/// Returns `None` only for prompts it does not recognise (so callers can warn).
pub fn translate_choice_prompt_en_to_ja(en: &str) -> Option<String> {
    let s = en.trim();
    if let Some(rest) = s.strip_prefix("Select up to ") {
        let num_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        let n: &str = &rest[..num_end];
        let after = &rest[num_end..];
        if after.contains("to keep") {
            return Some(format!("最大{}枚まで手札に残すカードを選択", n));
        }
        if after.contains("member(s) to change state") {
            return Some(format!("状態を変更するメンバーを最大{}体選択", n));
        }
        if after.contains("looked-at cards") {
            return Some(format!("見たカードから最大{}枚を選択（スキップ可）", n));
        }
    }
    if let Some(rest) = s.strip_prefix("Place up to ") {
        let num_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        let n: &str = &rest[..num_end];
        if s.contains("bottom of deck") {
            if n == "1" {
                return Some("見たカードをデッキの下に置きますか？".to_string());
            }
            return Some(format!("見たカードを最大{}枚までデッキの下に置きますか？", n));
        }
        if s.contains("top of deck") {
            if n == "1" {
                return Some("見たカードをデッキの上に置きますか？".to_string());
            }
            return Some(format!("見たカードを最大{}枚までデッキの上に置きますか？", n));
        }
    }
    if let Some(rest) = s.strip_prefix("Add up to ") {
        let num_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        let n: &str = &rest[..num_end];
        if s.contains("to hand") {
            if n == "1" {
                return Some("見たカードを手札に加えますか？".to_string());
            }
            return Some(format!("見たカードを最大{}枚まで手札に加えますか？", n));
        }
    }
    if let Some(rest) = s.strip_prefix("Discard up to ") {
        let num_end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        let n: &str = &rest[..num_end];
        if s.contains("looked-at") {
            if n == "1" {
                return Some("見たカードを控え室に置きますか？".to_string());
            }
            return Some(format!("見たカードを最大{}枚まで控え室に置きますか？", n));
        }
    }
    match s {
        "Repeat effect?" => Some("効果を繰り返しますか？".to_string()),
        "Pay optional cost or skip" => Some("オプションコストを支払うかスキップ".to_string()),
        _ => None,
    }
}
