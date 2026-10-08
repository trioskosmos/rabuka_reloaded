//! `describe_effect` — the 44-action rendering match.
//!
//! One match, not two: `describe_effect_en` / `describe_effect_ja` used to be
//! hand-maintained arm-for-arm duplicates over the same 44 wire strings, in
//! different orders (`reduce_live_card_set_limit` sat at position 43 in one and
//! 21 in the other). Adding an action type to one silently left the other
//! falling through to `effect.text`, so a new ability would describe itself in
//! English inside the Japanese output with no compile error and no test
//! failure. The sentence structure is genuinely per-language (Japanese needs
//! counters and particles where English needs articles), so both bodies survive
//! per arm; what is unified is the dispatch, the preamble, and the arm set.

#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

fn plural(n: u8, word: &str) -> String {
    if n == 1 {
        format!("{} {}", n, word)
    } else {
        format!("{} {}s", n, word)
    }
}

fn maybe_plural(count: Option<u8>, word: &str) -> String {
    plural(count.unwrap_or(1), word)
}

fn zone_label_inner(zone: Option<&str>, ja: bool) -> &str {
    if ja {
        match zone {
            Some("hand") => "手札",
            Some("discard") | Some("waitroom") => "控え室",
            Some("deck") => "デッキ",
            Some("deck_top") => "デッキの上",
            Some("deck_bottom") => "デッキの下",
            Some("stage") => "ステージ",
            Some("energy") => "エネルギー",
            Some("energy_deck") => "エネルギーデッキ",
            Some("energy_zone") => "エネルギーゾーン",
            Some("success_zone") => "成功ライブカード置き場",
            Some("live_card_zone") => "ライブカードゾーン",
            Some("under_member") => "このメンバーの下",
            Some("revealed_cards") => "公開されたカード",
            Some("those_cards") => "それらのカード",
            Some("all_selected") => "選択したカード",
            Some(s) => s,
            None => "不明",
        }
    } else {
        match zone {
            Some("hand") => "hand",
            Some("discard") => "the waiting room",
            Some("deck") => "deck",
            Some("deck_top") => "top of deck",
            Some("deck_bottom") => "bottom of deck",
            Some("stage") => "stage",
            Some("energy") => "energy",
            Some("energy_deck") => "energy deck",
            Some("energy_zone") => "energy zone",
            Some("waitroom") => "wait room",
            Some("success_zone") => "success zone",
            Some("live_card_zone") => "live card zone",
            Some("under_member") => "under this member",
            Some("revealed_cards") => "revealed cards",
            Some("those_cards") => "those cards",
            Some("all_selected") => "selected cards",
            Some(s) => s,
            None => "unknown",
        }
    }
}

pub fn zone_label(zone: Option<&str>) -> &str {
    zone_label_inner(zone, false)
}

fn card_type_label_inner(ct: Option<&str>, ja: bool) -> &str {
    if ja {
        match ct {
            Some("member_card") => "メンバー",
            Some("live_card") => "ライブカード",
            Some("energy_card") => "エネルギー",
            Some("card") => "カード",
            Some(s) => s,
            None => "カード",
        }
    } else {
        match ct {
            Some("member_card") => "member",
            Some("live_card") => "live card",
            Some("energy_card") => "energy",
            Some("card") => "card",
            Some(s) => s,
            None => "card",
        }
    }
}

pub fn card_type_label(ct: Option<&str>) -> &str {
    card_type_label_inner(ct, false)
}

fn state_verb_inner(state: Option<&str>, ja: bool) -> &str {
    if ja {
        match state {
            Some("wait") => "ウェイト",
            Some("active") => "アクティブ",
            Some(s) => s,
            None => "状態変更",
        }
    } else {
        match state {
            Some("wait") => "Rest",
            Some("active") => "Activate",
            Some(s) => s,
            None => "Change state of",
        }
    }
}

fn resource_label_inner(r: Option<&str>, ja: bool) -> &str {
    if ja {
        match r {
            Some("blade") => "ブレード",
            Some("heart") => "ハート",
            Some(s) => s,
            None => "リソース",
        }
    } else {
        match r {
            Some("blade") => "blade",
            Some("heart") => "heart",
            Some(s) => s,
            None => "resource",
        }
    }
}

/// Render a `duration` code as printed text.
///
/// This table is WIDER than `ability::util::parse_duration`, on purpose: it also
/// labels forms that are not effect durations at all — `live_success` and `turn_end`
/// appear in other fields and reuse this renderer. Only the codes `parse_duration`
/// accepts are valid in an effect's `duration`, and the corpus currently uses
/// `live_end`, `as_long_as`, `this_turn` and `unless`.
///
/// The asymmetry matters in one direction: if the parser ever starts emitting
/// `live_start`, `turn_end`, or `turn` as an effect duration, this function will
/// happily render a label while `push_temporary_effect` refuses the code and the
/// effect is never registered. `corpus_decode_coverage_test::every_duration_in_the_
/// corpus_is_parseable` fails on exactly that, so the decision gets made rather than
/// discovered.
fn duration_label_inner(d: Option<&str>, ja: bool) -> &str {
    if ja {
        match d {
            Some("live_end") => "ライブ終了時まで",
            Some("live_start") => "このライブの間",
            Some("live_success") => "ライブ成功時",
            Some("turn_end") | Some("turn") => "ターン終了時まで",
            Some(s) => s,
            None => "",
        }
    } else {
        match d {
            Some("live_end") => "until end of live",
            Some("live_start") => "for this live",
            Some("live_success") => "on live success",
            Some("turn_end") | Some("turn") => "until end of turn",
            Some(s) => s,
            None => "",
        }
    }
}

fn group_label(gn: Option<&Vec<String>>) -> String {
    match gn {
        Some(v) if !v.is_empty() => format!(" {} group", v.join("/")),
        _ => String::new(),
    }
}

pub mod condition;
pub mod cost;
pub mod effect;
pub mod prompt;

pub fn zone_label_ja(zone: Option<&str>) -> &str {
    zone_label_inner(zone, true)
}

pub fn state_verb_ja(state: Option<&str>) -> &str {
    state_verb_inner(state, true)
}

pub fn resource_label_ja(r: Option<&str>) -> &str {
    resource_label_inner(r, true)
}

pub use effect::{describe_effect, describe_effect_en, describe_effect_ja};
pub use cost::{describe_cost, describe_cost_en, describe_cost_ja, describe_sequential_cost,
               describe_sequential_cost_en, describe_sequential_cost_ja};
pub use prompt::{translate_choice_prompt_en_to_ja, CHOICE_PROMPT_TEMPLATES_EN};
