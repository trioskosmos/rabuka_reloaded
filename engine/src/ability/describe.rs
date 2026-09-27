use crate::ability::enums::{ActionType, Zone};
use crate::card::AbilityEffect;
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

/// Describe one ability effect in the requested language.
///
/// This was two hand-maintained functions - `describe_effect_en` and
/// `describe_effect_ja` - with an arm-for-arm identical 44-arm match over the
/// same 44 wire strings. They were not always in the same order
/// (`reduce_live_card_set_limit` sat at position 43 in one and 21 in the
/// other), which is harmless for distinct literals but is the kind of drift
/// that precedes a real divergence.
///
/// The duplication had a concrete failure mode: add an action type to one
/// match and the other silently falls through to `_ => effect.text`, so a new
/// ability would describe itself in English inside the Japanese output (or the
/// reverse) with no compile error and no test failure. One match cannot drift.
///
/// Note the sentence structure is genuinely per-language, not a set of labels:
/// Japanese needs counters and particles where English needs articles, so both
/// bodies survive per arm. What is unified is the dispatch, the preamble, and
/// the set of arms.
pub fn describe_effect(effect: &AbilityEffect, ja: bool) -> String {
    let action = effect.action.to_str();
    let ct_binding = effect.card_type_any();
    let ct = card_type_label_inner(ct_binding.map(|ct| ct.as_card_str()), ja);
    let c = effect.count_any();
    let t = effect.target_any();
    let s = effect.source_any();
    let d = effect.destination.map(|z| z.as_str());
    let gn = group_label(effect.group_names_any());

    match action {
        "move_cards" => {
            let dest = zone_label_inner(d, ja);
            match s {
                Some("those_cards") | Some("all_selected") => {
                    if ja {
                        format!("選択したカードを{}に置く", dest)
                    } else {
                        format!("Move the selected card(s) to {}", dest)
                    }
                }
                _ => {
                    let src = zone_label_inner(s, ja);
                    let mut result = if ja {
                        let count_str = if c == Some(1) {
                            format!("{}の{}", 1, ct)
                        } else {
                            format!("{}枚の{}", c.unwrap_or(1), ct)
                        };
                        format!("{}を{}から{}に置く", count_str, src, dest)
                    } else {
                        format!("Place {} from {} to {}", maybe_plural(c, ct), src, dest)
                    };
                    if let Some("wait") = effect.state_change_any().as_deref() {
                        result += if ja { "（レスト）" } else { " (rest)" };
                    }
                    result
                }
            }
        }
        "draw_card" => {
            if ja {
                let count_str = if c == Some(1) {
                    "1枚".to_string()
                } else {
                    format!("{}枚", c.unwrap_or(1))
                };
                if let Some(src) = s {
                    format!("{}から{}引く", zone_label_inner(Some(src), true), count_str)
                } else {
                    format!("{}引く", count_str)
                }
            } else if let Some(src) = s {
                format!(
                    "Draw {} from {}",
                    maybe_plural(c, "card"),
                    zone_label_inner(Some(src), false)
                )
            } else {
                maybe_plural(c, "card")
            }
        }

        "gain_resource" => {
            let r_binding = effect.resource_any();
            let r = resource_label_inner(r_binding.as_deref(), ja);
            let dur_binding = effect.duration_any();
            let dur = dur_binding.as_deref().and_then(|d| {
                let lbl = duration_label_inner(Some(d), ja);
                if lbl.is_empty() {
                    None
                } else {
                    Some(lbl)
                }
            });
            let dur_str = dur
                .map(|d| if ja { format!("（{}）", d) } else { format!(" {}", d) })
                .unwrap_or_default();
            let count_str = if ja {
                if c == Some(1) {
                    format!("{}", r)
                } else {
                    format!("{} {}", c.unwrap_or(1), r)
                }
            } else {
                maybe_plural(c, r)
            };
            match t {
                Some("opponent") if ja => format!("相手に{}を与える{}", count_str, dur_str),
                Some("opponent") => format!("Give {} to opponent{}", count_str, dur_str),
                _ if ja => format!("{}{}を得る{}", count_str, gn, dur_str),
                _ => format!("Gain {}{}{}", count_str, gn, dur_str),
            }
        }

        "change_state" => {
            let verb_binding = effect.state_change_any();
            let verb = state_verb_inner(verb_binding.as_deref(), ja);
            let cnt = c.unwrap_or(1);
            if ja {
                let who = match t {
                    Some("opponent") => "相手の",
                    _ => "",
                };
                let loc = s.map(|src| zone_label_inner(Some(src), true)).unwrap_or("");
                let lim = effect
                    .cost_limit_any()
                    .map(|cl| format!("（コスト{}以下）", cl))
                    .unwrap_or_default();
                if loc.is_empty() {
                    format!("{}を{}体{}{}にする{}", who, cnt, gn, verb, lim)
                } else {
                    format!("{}の{}{}体{}を{}にする{}", who, loc, cnt, gn, verb, lim)
                }
            } else {
                let who = match t {
                    Some("opponent") => "opponent ",
                    _ => "",
                };
                let loc = s
                    .map(|src| format!(" on {}", zone_label_inner(Some(src), false)))
                    .unwrap_or_default();
                let lim = effect
                    .cost_limit_any()
                    .map(|cl| format!(" (cost ≤ {})", cl))
                    .unwrap_or_default();
                format!("{} {}{}{} {}{}", verb, cnt, gn, loc, who, lim)
                    .trim()
                    .to_string()
            }
        }

        "modify_score" => {
            let val = effect.value_any().unwrap_or(1);
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("add");
            if ja {
                if op == "subtract" {
                    format!("スコアを{}減らす", val)
                } else {
                    format!("スコアを{}増やす", val)
                }
            } else if op == "subtract" {
                format!("Subtract {} from score", val)
            } else {
                format!("Add {} to score", val)
            }
        }

        "position_change" => {
            if let Some(ep) = effect.exclude_position_any().as_deref() {
                if ja {
                    format!("{}を避けてポジションチェンジ", ep)
                } else {
                    format!("Move a{} member away from {}", gn, ep)
                }
            } else if c == Some(1) || c.is_none() {
                if ja {
                    format!("ポジションチェンジ{}", gn)
                } else {
                    format!("Change position of a{} member", gn)
                }
            } else if ja {
                format!("{}体ポジションチェンジ{}", c.unwrap_or(1), gn)
            } else {
                format!("Change position of {}{} members", c.unwrap_or(1), gn)
            }
        }

        "select" | "select_cards" => {
            let src = zone_label_inner(s, ja);
            let opt = if effect.optional.unwrap_or(false) {
                if ja {
                    "（任意）"
                } else {
                    " (optional)"
                }
            } else {
                ""
            };
            if ja {
                format!(
                    "{}から{}を選ぶ{}",
                    src,
                    format!("{}枚の{}", c.unwrap_or(1), ct),
                    opt
                )
            } else {
                format!("Select {} from {}{}{}", maybe_plural(c, ct), src, gn, opt)
            }
        }

        "look_at" => {
            if ja {
                let count_str = if c == Some(1) {
                    "1枚".to_string()
                } else {
                    format!("{}枚", c.unwrap_or(1))
                };
                format!("{}のカードを{}見る", zone_label_inner(s, true), count_str)
            } else {
                format!(
                    "Look at {} from {}",
                    maybe_plural(c, "card"),
                    zone_label_inner(s, false)
                )
            }
        }

        "reveal" => {
            if ja {
                let count_str = if c == Some(1) {
                    "1枚".to_string()
                } else {
                    format!("{}枚", c.unwrap_or(1))
                };
                format!("{}のカードを{}公開する", zone_label_inner(s, true), count_str)
            } else {
                format!(
                    "Reveal {} from {}",
                    maybe_plural(c, "card"),
                    zone_label_inner(s, false)
                )
            }
        }

        "pay_energy" => {
            let opt = if effect.optional.unwrap_or(false) {
                if ja {
                    "（任意）"
                } else {
                    " (optional)"
                }
            } else {
                ""
            };
            if ja {
                format!("エネルギーを{}払う{}", c.unwrap_or(1), opt)
            } else {
                format!("Pay {} energy{}", c.unwrap_or(1), opt)
            }
        }

        "look_and_select" => {
            let look_count = effect.compound.look_action.as_ref().and_then(|a| a.count);
            let select_count = effect
                .compound
                .select_action
                .as_ref()
                .and_then(|a| a.count);
            let select_dest: Option<&str> = effect
                .compound
                .select_action
                .as_ref()
                .and_then(|a| a.destination.map(|z| z.as_str()))
                .map(|s| zone_label_inner(Some(s), ja));
            if let (Some(lc), Some(sc)) = (look_count, select_count) {
                if ja {
                    if sc == 1 {
                        format!("{}枚見て、1枚を{}に選ぶ", lc, select_dest.unwrap_or("手札"))
                    } else {
                        format!(
                            "{}枚見て、{}枚を{}に選ぶ",
                            lc,
                            sc,
                            select_dest.unwrap_or("手札")
                        )
                    }
                } else if sc == 1 {
                    format!(
                        "Look at {}; pick 1 to {}",
                        plural(lc, "card"),
                        select_dest.unwrap_or("hand")
                    )
                } else {
                    format!(
                        "Look at {}; pick {} to {}",
                        plural(lc, "card"),
                        sc,
                        select_dest.unwrap_or("hand")
                    )
                }
            } else if ja {
                "カードを見て選ぶ".to_string()
            } else {
                "Look at cards → pick".to_string()
            }
        }

        "restriction" => {
            let rt_binding = effect.restriction_type_any();
            let rt = rt_binding.unwrap_or(if ja { "制限" } else { "restriction" });
            if ja {
                match rt {
                    "cannot_baton_touch" => "バトンタッチでは控え室に置かれない".to_string(),
                    "cannot_activate" => "起動できない".to_string(),
                    "cannot_activate_by_effect" => "効果によってはアクティブにならない".to_string(),
                    "cannot_live" => "ライブできない".to_string(),
                    "cannot_place" => "成功ライブカード置き場に置くことができない".to_string(),
                    "cannot_wait_by_effect" => "相手の効果によってはウェイトしない".to_string(),
                    other => format!("{}制限を適用", other),
                }
            } else {
                match rt {
                    "cannot_baton_touch" => {
                        "Cannot be replaced by a baton touch (except from excluded groups)"
                            .to_string()
                    }
                    "cannot_activate" => "Cannot be activated".to_string(),
                    "cannot_activate_by_effect" => "Cannot be activated by card effects".to_string(),
                    "cannot_live" => "Cannot perform a live".to_string(),
                    "cannot_place" => "Cannot be placed in the success live card zone".to_string(),
                    "cannot_wait_by_effect" => {
                        "Do not go to Wait by opponent's effects".to_string()
                    }
                    other => format!("Restriction: {}", other),
                }
            }
        }

        "gain_ability" => match (ja, effect.ability_gain_any()) {
            (true, Some(ref ga)) => format!("アビリティを得る：{}", ga),
            (true, None) => "アビリティを得る".to_string(),
            (false, Some(ref ga)) => format!("Gain ability: {}", ga),
            (false, None) => "Gain ability".to_string(),
        },

        "do_nothing" => String::new(),

        "sequential" => {
            if let Some(ref actions) = effect.compound.actions {
                let parts: Vec<String> =
                    actions.iter().map(|a| describe_effect(a, ja)).collect();
                if parts.len() == 1 {
                    parts[0].clone()
                } else if ja {
                    format!("{}、その後{}", parts[..parts.len() - 1].join("、"), parts.last().unwrap())
                } else {
                    format!(
                        "{}; then {}",
                        parts[..parts.len() - 1].join("; "),
                        parts.last().unwrap()
                    )
                }
            } else {
                effect.text.to_string()
            }
        }

        "choice" => {
            if ja {
                "1つを選ぶ".to_string()
            } else {
                "Choose 1".to_string()
            }
        }

        "play_baton_touch" => {
            if ja {
                let suffix = if c == Some(1) {
                    String::new()
                } else {
                    format!("（{}体）", c.unwrap_or(1))
                };
                format!("バトンタッチ{}", suffix)
            } else {
                format!(
                    "Baton touch {} member{}",
                    c.unwrap_or(1),
                    if c == Some(1) { "" } else { "s" }
                )
            }
        }

        "modify_required_hearts" | "modify_required_hearts_global" => {
            let val = effect.value_any().unwrap_or(1);
            if val == 0 {
                if ja {
                    "必要ハートをクリア".to_string()
                } else {
                    "Clear required hearts".to_string()
                }
            } else if val > 0 {
                if ja {
                    format!("必要ハートを{}増やす", val)
                } else {
                    format!("Increase required hearts by {}", val)
                }
            } else if ja {
                format!("必要ハートを{}減らす", (val as i32).unsigned_abs())
            } else {
                format!(
                    "Decrease required hearts by {}",
                    (val as i32).unsigned_abs()
                )
            }
        }

        "activate_ability" => match (ja, effect.target_trigger_any()) {
            (true, Some(ref at)) => format!("{}アビリティを発動", at),
            (true, None) => "アビリティを発動".to_string(),
            (false, Some(ref at)) => format!("Activate {} ability", at),
            (false, None) => "Activate ability".to_string(),
        },

        "set_card_identity" => match (ja, effect.identities_any()) {
            (true, Some(ref ids)) => format!("扱い：{}", ids.join(", ")),
            (true, None) => "カードの扱いを設定".to_string(),
            (false, Some(ref ids)) => format!("Treat as: {}", ids.join(", ")),
            (false, None) => "Set card identity".to_string(),
        },

        "reduce_live_card_set_limit" => {
            if ja {
                format!(
                    "次のライブカードセットフェイズの上限を{}減らす",
                    c.unwrap_or(1)
                )
            } else {
                format!(
                    "Reduce the live card placement limit next Live Set phase by {}",
                    c.unwrap_or(1)
                )
            }
        }

        "set_blade_count" => {
            if ja {
                format!("ブレードを{}に設定", c.unwrap_or(0))
            } else {
                format!("Set blade to {}", c.unwrap_or(0))
            }
        }

        "set_heart_type" => match (ja, effect.heart_type_any()) {
            (true, Some(ref ht)) => format!("ハートタイプを{}に設定", ht),
            (true, None) => "ハートタイプを設定".to_string(),
            (false, Some(ref ht)) => format!("Set heart type to {}", ht),
            (false, None) => "Set heart type".to_string(),
        },

        "set_blade_type" => match (ja, effect.blade_type_any()) {
            (true, Some(ref bt)) => format!("ブレードタイプを{}に設定", bt),
            (true, None) => "ブレードタイプを設定".to_string(),
            (false, Some(ref bt)) => format!("Set blade type to {}", bt),
            (false, None) => "Set blade type".to_string(),
        },

        "discard_until_count" => {
            let n = effect.target_count_any().unwrap_or(1);
            if ja {
                format!("手札が{}枚になるまで捨てる", n)
            } else {
                format!(
                    "Discard down to {} card{} in hand",
                    n,
                    if effect.target_count_any() == Some(1) {
                        ""
                    } else {
                        "s"
                    }
                )
            }
        }

        "draw_until_count" => {
            let n = effect.target_count_any().unwrap_or(1);
            if ja {
                let from = s
                    .map(|src| format!("{}から", zone_label_inner(Some(src), true)))
                    .unwrap_or_default();
                format!("手札が{}枚になるまで{}引く", n, from)
            } else {
                let from = s
                    .map(|src| format!(" from {}", zone_label_inner(Some(src), false)))
                    .unwrap_or_default();
                format!(
                    "Draw until {} card{} in hand{}",
                    n,
                    if effect.target_count_any() == Some(1) {
                        ""
                    } else {
                        "s"
                    },
                    from
                )
            }
        }

        "modify_cost" => {
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("subtract");
            // Set-cost (「コストはNになる」) uses the absolute `value` field,
            // not the additive count — e.g. LL-bp7-001 sets cost to 10.
            if op == "set" {
                match effect.value_any().and_then(|v| i16::try_from(v).ok()) {
                    Some(v) if ja => format!("コストは{}になる", v),
                    Some(v) => format!("Cost becomes {}", v),
                    None if ja => "コストを設定".to_string(),
                    None => "Set cost".to_string(),
                }
            } else {
                let amt = c.unwrap_or(1);
                if ja {
                    if op == "subtract" {
                        format!("{{{{icon_energy.png|E}}}}を{}減らす", amt)
                    } else {
                        format!("{{{{icon_energy.png|E}}}}を{}増やす", amt)
                    }
                } else if op == "subtract" {
                    format!("Reduce cost by {}", amt)
                } else {
                    format!("Increase cost by {}", amt)
                }
            }
        }

        "modify_yell_count" => {
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("add");
            let amt = c.unwrap_or(1);
            if ja {
                if op == "subtract" {
                    format!("エール回数を{}減らす", amt)
                } else {
                    format!("エール回数を{}増やす", amt)
                }
            } else if op == "subtract" {
                format!("Reduce yell count by {}", amt)
            } else {
                format!("Increase yell count by {}", amt)
            }
        }

        "perform_yell" => {
            let amt = c.unwrap_or(1);
            let max = effect.repeat_limit_any().unwrap_or(amt);
            if ja {
                if amt == max {
                    format!("エールを{}回行う", amt)
                } else {
                    format!("最大{}回エールを行う", max)
                }
            } else if amt == max {
                format!("Perform {} yell{}", amt, if amt == 1 { "" } else { "s" })
            } else {
                format!(
                    "Perform up to {} yell{}",
                    max,
                    if max == 1 { "" } else { "s" }
                )
            }
        }

        "re_yell" => {
            let lose = effect.lose_blade_hearts_any().unwrap_or(false);
            if ja {
                let suffix = if lose { "（ブレード/ハートを失う）" } else { "" };
                format!("再エール{}", suffix)
            } else {
                format!("Re-yell{}", if lose { " (lose blade/hearts)" } else { "" })
            }
        }

        "specify_heart_color" => {
            if ja {
                "ハートの色を指定する".to_string()
            } else {
                "Choose a heart color".to_string()
            }
        }

        "place_energy_under_member" => {
            let e = effect.energy_count_any().or(c).unwrap_or(1);
            if ja {
                format!("このメンバーの下にエネルギーを{}枚置く", e)
            } else {
                format!("Place {} energy under this member", e)
            }
        }

        "invalidate_ability" => {
            if ja {
                "アビリティを無効にする".to_string()
            } else {
                "Cancel an ability".to_string()
            }
        }

        "choose_target_player" => {
            if ja {
                "自分か相手を選ぶ".to_string()
            } else {
                "Choose self or opponent".to_string()
            }
        }

        "repeat_procedure" => {
            let max = effect.repeat_limit_any().unwrap_or(c.unwrap_or(1));
            if ja {
                format!("最大{}回繰り返す", max)
            } else {
                format!("Repeat up to {} times", max)
            }
        }

        "sequential_cost" => {
            if let Some(ref costs) = effect.compound.actions {
                let parts: Vec<String> = costs.iter().map(|c| describe_cost(c, ja)).collect();
                let combined = parts.join("\n");
                if costs.last().and_then(|c| c.optional).unwrap_or(false) {
                    if ja {
                        format!("{}\n（またはスキップ）", combined)
                    } else {
                        format!("{}\n(or skip)", combined)
                    }
                } else {
                    combined
                }
            } else {
                effect.text.to_string()
            }
        }

        "reveal_until_live_card" => {
            if ja {
                "ライブカードが出るまでデッキを公開する".to_string()
            } else {
                "Reveal from deck until a live card appears".to_string()
            }
        }

        "gain_ability_from_source" => {
            if ja {
                "アビリティをコピーする".to_string()
            } else {
                "Copy an ability".to_string()
            }
        }

        "conditional_on_optional" => {
            let parts: Vec<String> = [effect
                .compound
                .conditional_action
                .as_ref()
                .map(|a| describe_effect(a, ja))]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                if ja {
                    "コストを払うかスキップ".to_string()
                } else {
                    "Pay cost or skip".to_string()
                }
            } else if ja {
                format!("コストを払うかスキップ：{}", parts.join("、"))
            } else {
                format!("Pay cost or skip: {}", parts.join("; "))
            }
        }

        "conditional_on_result" => {
            let parts: Vec<String> = [
                effect
                    .compound
                    .primary_effect
                    .as_ref()
                    .map(|a| describe_effect(a, ja)),
                effect
                    .compound
                    .followup_action
                    .as_ref()
                    .map(|a| describe_effect(a, ja)),
            ]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                effect.text.to_string()
            } else if ja {
                parts.join("、その後")
            } else {
                parts.join("; then ")
            }
        }

        "conditional_alternative" => {
            let primary = effect
                .compound
                .primary_effect
                .as_ref()
                .map(|a| describe_effect(a, ja));
            let alt = effect
                .alternative_effect_any()
                .as_ref()
                .map(|a| describe_effect(a, ja));
            match (primary, alt) {
                (Some(p), Some(a)) if ja => format!("どちらか：{} / または：{}", p, a),
                (Some(p), Some(a)) => format!("Either: {} / Or: {}", p, a),
                (Some(p), None) => p,
                (None, Some(a)) => a,
                (None, None) => effect.text.to_string(),
            }
        }

        "select_number" => {
            if ja {
                format!("数を{}つ選ぶ", c.unwrap_or(1))
            } else {
                format!("Choose a number ({})", c.unwrap_or(1))
            }
        }

        "modify_yell_source" => {
            let bottom = effect.yell_source_any().as_deref() == Some("deck_bottom");
            if ja {
                if bottom {
                    "エールはデッキの下から行う".to_string()
                } else {
                    "エールの行う場所を変更".to_string()
                }
            } else if bottom {
                "Perform yells from the bottom of the deck instead of the top".to_string()
            } else {
                "Modify the yell source".to_string()
            }
        }

        "suppress_ability_trigger" => {
            let live_start = effect.suppressed_trigger_any().as_deref() == Some("live_start");
            match (ja, live_start) {
                (true, true) => "ライブ開始時能力は発動しない".to_string(),
                (true, false) => "能力は発動しない".to_string(),
                (false, true) => "Live Start abilities do not activate".to_string(),
                (false, false) => "Abilities do not activate".to_string(),
            }
        }

        _ => effect.text.to_string(),
    }
}

/// Describe one cost item in the requested language.
pub fn describe_cost(cost: &AbilityEffect, ja: bool) -> String {
    match cost.action {
        ActionType::PayEnergy => {
            let count = cost.energy_count_any().unwrap_or(1);
            if ja {
                format!("{{{{icon_energy.png|E}}}}を{}払う", count)
            } else {
                format!("Pay {} energy", count)
            }
        }
        ActionType::ChangeState => {
            if cost.self_cost_any() == Some(true) {
                let state_binding = cost.state_change_any();
                let state = state_binding.unwrap_or("wait");
                match (ja, state) {
                    (true, "wait") => "このメンバーをウェイト".to_string(),
                    (true, s) => format!("このメンバーを{}にする", s),
                    (false, "wait") => "Rest this member".to_string(),
                    (false, s) => format!("Change this member to {}", s),
                }
            } else {
                describe_effect(cost, ja)
            }
        }
        ActionType::MoveCards => {
            let src = zone_label_inner(cost.source.map(|z| z.as_str()), ja);
            let dest = zone_label_inner(cost.destination.map(|z| z.as_str()), ja);
            let card_type_binding = cost.card_type_any();
            let card_type = card_type_label_inner(card_type_binding.map(|ct| ct.as_card_str()), ja);
            let count = cost.count.unwrap_or(1);
            if cost.self_cost_any() == Some(true) && cost.source == Some(Zone::ThoseCards) {
                if ja {
                    format!("そのカードを{}に置く", dest)
                } else {
                    format!("Move that card to {}", dest)
                }
            } else if ja {
                let count_str = if count == 1 {
                    format!("{}の{}", 1, card_type)
                } else {
                    format!("{}枚の{}", count, card_type)
                };
                format!("{}を{}から{}に置く", count_str, src, dest)
            } else {
                format!("Place {} {} from {} to {}", count, card_type, src, dest)
            }
        }
        ActionType::Reveal => {
            let count = cost.count.unwrap_or(1);
            let source = cost.source.map(|z| z.as_str()).unwrap_or("hand");
            if ja {
                let count_str = if count == 1 {
                    "1枚".to_string()
                } else {
                    format!("{}枚", count)
                };
                format!("{}を{}から公開する", count_str, zone_label_inner(Some(source), true))
            } else {
                format!(
                    "Reveal {} card(s) from {}",
                    count,
                    zone_label_inner(Some(source), false)
                )
            }
        }
        _ => describe_effect(cost, ja),
    }
}

/// Combined description of a `sequential_cost` up to the choice sub-cost.
/// Binary costs before the choice are included; the choice cost's
/// "(or skip)" is appended at the end if the choice sub-cost is optional.
pub fn describe_sequential_cost(costs: &[Box<AbilityEffect>], choice_index: usize, ja: bool) -> String {
    let parts: Vec<String> = (0..=choice_index)
        .map(|i| describe_cost(&costs[i], ja))
        .collect();
    let combined = parts.join("\n");
    if costs[choice_index].optional.unwrap_or(false) {
        if ja {
            format!("{}（またはスキップ）", combined)
        } else {
            format!("{}\n(or skip)", combined)
        }
    } else {
        combined
    }
}

pub fn describe_effect_en(effect: &AbilityEffect) -> String {
    describe_effect(effect, false)
}

pub fn describe_effect_ja(effect: &AbilityEffect) -> String {
    describe_effect(effect, true)
}

pub fn describe_cost_en(cost: &AbilityEffect) -> String {
    describe_cost(cost, false)
}

pub fn describe_cost_ja(cost: &AbilityEffect) -> String {
    describe_cost(cost, true)
}

pub fn describe_sequential_cost_en(costs: &[Box<AbilityEffect>], choice_index: usize) -> String {
    describe_sequential_cost(costs, choice_index, false)
}

pub fn describe_sequential_cost_ja(costs: &[Box<AbilityEffect>], choice_index: usize) -> String {
    describe_sequential_cost(costs, choice_index, true)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_prompt_templates_all_have_japanese() {
        for tpl in CHOICE_PROMPT_TEMPLATES_EN {
            assert!(
                translate_choice_prompt_en_to_ja(tpl).is_some(),
                "choice prompt template missing Japanese translation: {:?}",
                tpl
            );
        }
    }

    #[test]
    fn choice_prompt_translator_handles_parameterized_prompts() {
        assert_eq!(
            translate_choice_prompt_en_to_ja("Select up to 3 card(s) to keep"),
            Some("最大3枚まで手札に残すカードを選択".to_string())
        );
        assert_eq!(
            translate_choice_prompt_en_to_ja("Select up to 1 member(s) to change state"),
            Some("状態を変更するメンバーを最大1体選択".to_string())
        );
        assert_eq!(
            translate_choice_prompt_en_to_ja("Select up to 3 card(s) from the 5 looked-at cards (or skip)"),
            Some("見たカードから最大3枚を選択（スキップ可）".to_string())
        );
        assert_eq!(
            translate_choice_prompt_en_to_ja("Repeat effect?"),
            Some("効果を繰り返しますか？".to_string())
        );
        assert_eq!(translate_choice_prompt_en_to_ja("Some unrelated text"), None);
    }
}
