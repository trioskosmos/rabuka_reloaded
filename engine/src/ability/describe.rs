use crate::card::AbilityEffect;
#[cfg(feature = "psp")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

fn plural(n: u32, word: &str) -> String {
    if n == 1 {
        format!("{} {}", n, word)
    } else {
        format!("{} {}s", n, word)
    }
}

fn maybe_plural(count: Option<u32>, word: &str) -> String {
    match count {
        Some(1) => format!("1 {}", word),
        Some(n) => format!("{} {}s", n, word),
        None => format!("1 {}", word),
    }
}

pub fn zone_label(zone: Option<&str>) -> &str {
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

pub fn card_type_label(ct: Option<&str>) -> &str {
    match ct {
        Some("member_card") => "member",
        Some("live_card") => "live card",
        Some("energy_card") => "energy",
        Some("card") => "card",
        Some(s) => s,
        None => "card",
    }
}

fn state_verb(state: Option<&str>) -> &str {
    match state {
        Some("wait") => "Rest",
        Some("active") => "Activate",
        Some(s) => s,
        None => "Change state of",
    }
}

fn resource_label(r: Option<&str>) -> &str {
    match r {
        Some("blade") => "blade",
        Some("heart") => "heart",
        Some(s) => s,
        None => "resource",
    }
}

fn duration_label(d: Option<&str>) -> &str {
    match d {
        Some("live_end") => "until end of live",
        Some("live_start") => "for this live",
        Some("live_success") => "on live success",
        Some("turn_end") | Some("turn") => "until end of turn",
        Some(s) => s,
        None => "",
    }
}

fn group_label(gn: Option<&Vec<String>>) -> String {
    match gn {
        Some(v) if !v.is_empty() => format!(" {} group", v.join("/")),
        _ => String::new(),
    }
}

pub fn describe_effect_en(effect: &AbilityEffect) -> String {
    let action = effect.action.as_str();
    let ct_binding = effect.card_type_any();
    let ct = card_type_label(ct_binding.as_deref());
    let c = effect.count_any();
    let t = effect.target_any();
    let s = effect.source_any();
    let d = effect.destination.as_deref();
    let gn = group_label(effect.group_names_any());

    match action {
        "move_cards" => {
            let dest = zone_label(d);
            match s {
                Some("those_cards") | Some("all_selected") => {
                    format!("Move the selected card(s) to {}", dest)
                }
                _ => {
                    let src = zone_label(s);
                    let mut result =
                        format!("Place {} from {} to {}", maybe_plural(c, ct), src, dest);
                    if let Some("wait") = effect.state_change_any().as_deref() {
                        result += " (rest)";
                    }
                    result
                }
            }
        }
        "draw_card" => {
            if let Some(src) = s {
                format!(
                    "Draw {} from {}",
                    maybe_plural(c, "card"),
                    zone_label(Some(src))
                )
            } else {
                maybe_plural(c, "card")
            }
        }

        "gain_resource" => {
            let r_binding = effect.resource_any();
            let r = resource_label(r_binding.as_deref());
            let dur_binding = effect.duration_any();
            let dur = dur_binding.as_deref().and_then(|d| {
                let lbl = duration_label(Some(d));
                if lbl.is_empty() {
                    None
                } else {
                    Some(lbl)
                }
            });
            let dur_str = dur.map(|d| format!(" {}", d)).unwrap_or_default();
            let count_str = maybe_plural(c, r);
            match t {
                Some("opponent") => format!("Give {} to opponent{}", count_str, dur_str),
                _ => format!("Gain {}{}{}", count_str, gn, dur_str),
            }
        }

        "change_state" => {
            let verb_binding = effect.state_change_any();
            let verb = state_verb(verb_binding.as_deref());
            let cnt = c.unwrap_or(1);
            let who = match t {
                Some("opponent") => "opponent ",
                _ => "",
            };
            let loc = s
                .map(|src| format!(" on {}", zone_label(Some(src))))
                .unwrap_or_default();
            let lim = effect
                .cost_limit_any()
                .map(|cl| format!(" (cost ≤ {})", cl))
                .unwrap_or_default();
            format!("{} {}{}{} {}{}", verb, cnt, gn, loc, who, lim)
                .trim()
                .to_string()
        }

        "modify_score" => {
            let val = effect.value_any().unwrap_or(1);
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("add");
            if op == "subtract" {
                format!("Subtract {} from score", val)
            } else {
                format!("Add {} to score", val)
            }
        }

        "position_change" => {
            if let Some(ep) = effect.exclude_position_any().as_deref() {
                format!("Move a{} member away from {}", gn, ep)
            } else if c == Some(1) || c.is_none() {
                format!("Change position of a{} member", gn)
            } else {
                format!("Change position of {}{} members", c.unwrap_or(1), gn)
            }
        }

        "select" | "select_cards" => {
            let src = zone_label(s);
            let opt = if effect.optional.unwrap_or(false) {
                " (optional)"
            } else {
                ""
            };
            format!("Select {} from {}{}{}", maybe_plural(c, ct), src, gn, opt)
        }

        "look_at" => format!("Look at {} from {}", maybe_plural(c, "card"), zone_label(s)),

        "reveal" => format!("Reveal {} from {}", maybe_plural(c, "card"), zone_label(s)),

        "pay_energy" => {
            let opt = if effect.optional.unwrap_or(false) {
                " (optional)"
            } else {
                ""
            };
            format!("Pay {} energy{}", c.unwrap_or(1), opt)
        }

        "look_and_select" => {
            let look_count = effect.compound.look_action.as_ref().and_then(|a| a.count);
            let select_count = effect.compound.select_action.as_ref().and_then(|a| a.count);
            let select_dest: Option<&str> = effect
                .compound
                .select_action
                .as_ref()
                .and_then(|a| a.destination.as_deref())
                .map(|s| zone_label(Some(s)));
            if let (Some(lc), Some(sc)) = (look_count, select_count) {
                if sc == 1 {
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
            } else {
                "Look at cards → pick".to_string()
            }
        }

        "restriction" => {
            let rt_binding = effect.restriction_type_any();
            let rt = rt_binding.unwrap_or("restriction");
            format!("Apply {} restriction", rt)
        }

        "gain_ability" => {
            if let Some(ref ga) = effect.ability_gain_any() {
                format!("Gain ability: {}", ga)
            } else {
                "Gain ability".to_string()
            }
        }

        "do_nothing" => String::new(),

        "sequential" => {
            if let Some(ref actions) = effect.compound.actions {
                let parts: Vec<String> = actions.iter().map(|a| describe_effect_en(a)).collect();
                if parts.len() == 1 {
                    parts[0].clone()
                } else {
                    format!(
                        "{}; then {}",
                        parts[..parts.len() - 1].join("; "),
                        parts.last().unwrap()
                    )
                }
            } else {
                effect.text.clone()
            }
        }

        "choice" => "Choose 1".to_string(),

        "play_baton_touch" => format!(
            "Baton touch {} member{}",
            c.unwrap_or(1),
            if c == Some(1) { "" } else { "s" }
        ),

        "modify_required_hearts" | "modify_required_hearts_global" => {
            let val = effect.value_any().unwrap_or(1);
            if val == 0 {
                "Clear required hearts".to_string()
            } else if val > 0 {
                format!("Increase required hearts by {}", val)
            } else {
                format!(
                    "Decrease required hearts by {}",
                    (val as i32).unsigned_abs()
                )
            }
        }

        "activate_ability" => {
            if let Some(ref at) = effect.target_trigger_any() {
                format!("Activate {} ability", at)
            } else {
                "Activate ability".to_string()
            }
        }

        "set_card_identity" => {
            if let Some(ref ids) = effect.identities_any() {
                format!("Treat as: {}", ids.join(", "))
            } else {
                "Set card identity".to_string()
            }
        }

        "set_blade_count" => format!("Set blade to {}", c.unwrap_or(0)),

        "set_heart_type" => {
            if let Some(ref ht) = effect.heart_type_any() {
                format!("Set heart type to {}", ht)
            } else {
                "Set heart type".to_string()
            }
        }

        "set_blade_type" => {
            if let Some(ref bt) = effect.blade_type_any() {
                format!("Set blade type to {}", bt)
            } else {
                "Set blade type".to_string()
            }
        }

        "discard_until_count" => {
            format!(
                "Discard down to {} card{} in hand",
                effect.target_count_any().unwrap_or(1),
                if effect.target_count_any() == Some(1) {
                    ""
                } else {
                    "s"
                }
            )
        }

        "draw_until_count" => {
            let from = s
                .map(|src| format!(" from {}", zone_label(Some(src))))
                .unwrap_or_default();
            format!(
                "Draw until {} card{} in hand{}",
                effect.target_count_any().unwrap_or(1),
                if effect.target_count_any() == Some(1) {
                    ""
                } else {
                    "s"
                },
                from
            )
        }

        "modify_cost" => {
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("subtract");
            let amt = c.unwrap_or(1);
            if op == "subtract" {
                format!("Reduce cost by {}", amt)
            } else {
                format!("Increase cost by {}", amt)
            }
        }

        "modify_yell_count" => {
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("add");
            let amt = c.unwrap_or(1);
            if op == "subtract" {
                format!("Reduce yell count by {}", amt)
            } else {
                format!("Increase yell count by {}", amt)
            }
        }

        "perform_yell" => {
            let amt = c.unwrap_or(1);
            let max = effect.repeat_limit_any().unwrap_or(amt);
            if amt == max {
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
            format!(
                "Re-yell{}",
                if effect.lose_blade_hearts_any().unwrap_or(false) {
                    " (lose blade/hearts)"
                } else {
                    ""
                }
            )
        }

        "specify_heart_color" => "Choose a heart color".to_string(),

        "place_energy_under_member" => {
            let e = effect.energy_count_any().or(c).unwrap_or(1);
            format!("Place {} energy under this member", e)
        }

        "invalidate_ability" => "Cancel an ability".to_string(),

        "choose_target_player" => "Choose self or opponent".to_string(),

        "repeat_procedure" => {
            let max = effect.repeat_limit_any().unwrap_or(c.unwrap_or(1));
            format!("Repeat up to {} times", max)
        }

        "sequential_cost" => {
            if let Some(ref costs) = effect.compound.actions {
                let parts: Vec<String> = costs.iter().map(|c| describe_cost_en(c)).collect();
                let combined = parts.join("\n");
                if costs.last().and_then(|c| c.optional).unwrap_or(false) {
                    format!("{}\n(or skip)", combined)
                } else {
                    combined
                }
            } else {
                effect.text.clone()
            }
        }

        "reveal_until_live_card" => "Reveal from deck until a live card appears".to_string(),

        "gain_ability_from_source" => "Copy an ability".to_string(),

        "conditional_on_optional" => {
            let parts: Vec<String> = [effect
                .compound
                .conditional_action
                .as_ref()
                .map(|a| describe_effect_en(a))]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                "Pay cost or skip".to_string()
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
                    .map(|a| describe_effect_en(a)),
                effect
                    .compound
                    .followup_action
                    .as_ref()
                    .map(|a| describe_effect_en(a)),
            ]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                effect.text.clone()
            } else {
                parts.join("; then ")
            }
        }

        "conditional_alternative" => {
            let primary = effect
                .compound
                .primary_effect
                .as_ref()
                .map(|a| describe_effect_en(a));
            let alt = effect
                .alternative_effect_any()
                .as_ref()
                .map(|a| describe_effect_en(a));
            match (primary, alt) {
                (Some(p), Some(a)) => format!("Either: {} / Or: {}", p, a),
                (Some(p), None) => p,
                (None, Some(a)) => a,
                (None, None) => effect.text.clone(),
            }
        }

        _ => effect.text.clone(),
    }
}

/// Describe a single cost item in English (used for combined cost prompts).
pub fn describe_cost_en(cost: &AbilityEffect) -> String {
    match cost.action.as_str() {
        "pay_energy" => {
            let count = cost.energy_count_any().unwrap_or(1);
            format!("Pay {} energy", count)
        }
        "change_state" => {
            if cost.self_cost_any() == Some(true) {
                let state_binding = cost.state_change_any();
                let state = state_binding.unwrap_or("wait");
                match state {
                    "wait" => "Rest this member".to_string(),
                    s => format!("Change this member to {}", s),
                }
            } else {
                describe_effect_en(cost)
            }
        }
        "move_cards" => {
            let src = zone_label(cost.source.as_deref());
            let dest = zone_label(cost.destination.as_deref());
            let card_type_binding = cost.card_type_any();
            let card_type = card_type_label(card_type_binding.as_deref());
            let count = cost.count.unwrap_or(1);
            if cost.self_cost_any() == Some(true) && cost.source.as_deref() == Some("those_cards") {
                format!("Move that card to {}", dest)
            } else {
                format!("Place {} {} from {} to {}", count, card_type, src, dest)
            }
        }
        "reveal" => {
            let count = cost.count.unwrap_or(1);
            let source = cost.source.as_deref().unwrap_or("hand");
            format!("Reveal {} card(s) from {}", count, zone_label(Some(source)))
        }
        _ => describe_effect_en(cost),
    }
}

/// Describe a single cost item in Japanese.
pub fn describe_cost_ja(cost: &AbilityEffect) -> String {
    match cost.action.as_str() {
        "pay_energy" => {
            let count = cost.energy_count_any().unwrap_or(1);
            format!("{{{{icon_energy.png|E}}}}を{}払う", count)
        }
        "change_state" => {
            if cost.self_cost_any() == Some(true) {
                let state_binding = cost.state_change_any();
                let state = state_binding.unwrap_or("wait");
                match state {
                    "wait" => "このメンバーをウェイト".to_string(),
                    s => format!("このメンバーを{}にする", s),
                }
            } else {
                describe_effect_ja(cost)
            }
        }
        "move_cards" => {
            let src = zone_label_ja(cost.source.as_deref());
            let dest = zone_label_ja(cost.destination.as_deref());
            let ct_binding = cost.card_type_any();
            let ct = card_type_label_ja(ct_binding.as_deref());
            let count = cost.count.unwrap_or(1);
            if cost.self_cost_any() == Some(true) && cost.source.as_deref() == Some("those_cards") {
                format!("そのカードを{}に置く", dest)
            } else {
                let count_str = if count == 1 {
                    format!("{}の{}", 1, ct)
                } else {
                    format!("{}枚の{}", count, ct)
                };
                format!("{}を{}から{}に置く", count_str, src, dest)
            }
        }
        "reveal" => {
            let count = cost.count.unwrap_or(1);
            let source = cost.source.as_deref().unwrap_or("hand");
            let count_str = if count == 1 {
                "1枚".to_string()
            } else {
                format!("{}枚", count)
            };
            format!("{}を{}から公開する", count_str, zone_label_ja(Some(source)))
        }
        _ => describe_effect_ja(cost),
    }
}

/// Build a combined English description for a sequential_cost up to the choice sub-cost.
/// Binary costs before the choice are included; the choice cost's "(or skip)" is appended
/// at the end if the choice sub-cost is optional.
pub fn describe_sequential_cost_en(costs: &[Box<AbilityEffect>], choice_index: usize) -> String {
    let parts: Vec<String> = (0..=choice_index)
        .map(|i| describe_cost_en(&costs[i]))
        .collect();
    let combined = parts.join("\n");
    if costs[choice_index].optional.unwrap_or(false) {
        format!("{}\n(or skip)", combined)
    } else {
        combined
    }
}

/// Build a combined Japanese description for a sequential_cost up to the choice sub-cost.
pub fn describe_sequential_cost_ja(costs: &[Box<AbilityEffect>], choice_index: usize) -> String {
    let parts: Vec<String> = (0..=choice_index)
        .map(|i| describe_cost_ja(&costs[i]))
        .collect();
    let combined = parts.join("\n");
    if costs[choice_index].optional.unwrap_or(false) {
        format!("{}（またはスキップ）", combined)
    } else {
        combined
    }
}

// ── Japanese description ──────────────────────────────────────────────

pub fn zone_label_ja(zone: Option<&str>) -> &str {
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
}

fn card_type_label_ja(ct: Option<&str>) -> &str {
    match ct {
        Some("member_card") => "メンバー",
        Some("live_card") => "ライブカード",
        Some("energy_card") => "エネルギー",
        Some("card") => "カード",
        Some(s) => s,
        None => "カード",
    }
}

fn state_verb_ja(state: Option<&str>) -> &str {
    match state {
        Some("wait") => "ウェイト",
        Some("active") => "アクティブ",
        Some(s) => s,
        None => "状態変更",
    }
}

fn resource_label_ja(r: Option<&str>) -> &str {
    match r {
        Some("blade") => "ブレード",
        Some("heart") => "ハート",
        Some(s) => s,
        None => "リソース",
    }
}

fn duration_label_ja(d: Option<&str>) -> &str {
    match d {
        Some("live_end") => "ライブ終了時まで",
        Some("live_start") => "このライブの間",
        Some("live_success") => "ライブ成功時",
        Some("turn_end") | Some("turn") => "ターン終了時まで",
        Some(s) => s,
        None => "",
    }
}

pub fn describe_effect_ja(effect: &AbilityEffect) -> String {
    let action = effect.action.as_str();
    let ct_binding = effect.card_type_any();
    let ct = card_type_label_ja(ct_binding.as_deref());
    let c = effect.count_any();
    let t = effect.target_any();
    let s = effect.source_any();
    let d = effect.destination.as_deref();
    let gn = group_label(effect.group_names_any());

    match action {
        "move_cards" => {
            let dest = zone_label_ja(d);
            match s {
                Some("those_cards") | Some("all_selected") => {
                    format!("選択したカードを{}に置く", dest)
                }
                _ => {
                    let src = zone_label_ja(s);
                    let mut result = format!(
                        "{}を{}から{}に置く",
                        if c == Some(1) {
                            format!("{}の{}", 1, ct).to_string()
                        } else {
                            format!("{}枚の{}", c.unwrap_or(1), ct)
                        },
                        src,
                        dest
                    );
                    if let Some("wait") = effect.state_change_any().as_deref() {
                        result += "（レスト）";
                    }
                    result
                }
            }
        }
        "draw_card" => {
            let count_str = if c == Some(1) {
                "1枚".to_string()
            } else {
                format!("{}枚", c.unwrap_or(1))
            };
            if let Some(src) = s {
                format!("{}から{}引く", zone_label_ja(Some(src)), count_str)
            } else {
                format!("{}引く", count_str)
            }
        }
        "gain_resource" => {
            let r_binding = effect.resource_any();
            let r = resource_label_ja(r_binding.as_deref());
            let dur_binding = effect.duration_any();
            let dur = dur_binding.as_deref().and_then(|d| {
                let lbl = duration_label_ja(Some(d));
                if lbl.is_empty() {
                    None
                } else {
                    Some(lbl)
                }
            });
            let dur_str = dur.map(|d| format!("（{}）", d)).unwrap_or_default();
            let count_str = if c == Some(1) {
                format!("{}", r)
            } else {
                format!("{} {}", c.unwrap_or(1), r)
            };
            match t {
                Some("opponent") => format!("相手に{}を与える{}", count_str, dur_str),
                _ => format!("{}{}を得る{}", count_str, gn, dur_str),
            }
        }
        "change_state" => {
            let verb_binding = effect.state_change_any();
            let verb = state_verb_ja(verb_binding.as_deref());
            let cnt = c.unwrap_or(1);
            let who = match t {
                Some("opponent") => "相手の",
                _ => "",
            };
            let loc = s.map(|src| zone_label_ja(Some(src))).unwrap_or("");
            let lim = effect
                .cost_limit_any()
                .map(|cl| format!("（コスト{}以下）", cl))
                .unwrap_or_default();
            if loc.is_empty() {
                format!("{}を{}体{}{}にする{}", who, cnt, gn, verb, lim)
            } else {
                format!("{}の{}{}体{}を{}にする{}", who, loc, cnt, gn, verb, lim)
            }
        }
        "modify_score" => {
            let val = effect.value_any().unwrap_or(1);
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("add");
            if op == "subtract" {
                format!("スコアを{}減らす", val)
            } else {
                format!("スコアを{}増やす", val)
            }
        }
        "position_change" => {
            if let Some(ep) = effect.exclude_position_any().as_deref() {
                format!("{}を避けてポジションチェンジ", ep)
            } else if c == Some(1) || c.is_none() {
                format!("ポジションチェンジ{}", gn)
            } else {
                format!("{}体ポジションチェンジ{}", c.unwrap_or(1), gn)
            }
        }
        "select" | "select_cards" => {
            let src = zone_label_ja(s);
            let opt = if effect.optional.unwrap_or(false) {
                "（任意）"
            } else {
                ""
            };
            format!(
                "{}から{}を選ぶ{}",
                src,
                if c == Some(1) {
                    format!("{}枚の{}", 1, ct)
                } else {
                    format!("{}枚の{}", c.unwrap_or(1), ct)
                },
                opt
            )
        }
        "look_at" => {
            let count_str = if c == Some(1) {
                "1枚".to_string()
            } else {
                format!("{}枚", c.unwrap_or(1))
            };
            format!("{}のカードを{}見る", zone_label_ja(s), count_str)
        }
        "reveal" => {
            let count_str = if c == Some(1) {
                "1枚".to_string()
            } else {
                format!("{}枚", c.unwrap_or(1))
            };
            format!("{}のカードを{}公開する", zone_label_ja(s), count_str)
        }
        "pay_energy" => {
            let opt = if effect.optional.unwrap_or(false) {
                "（任意）"
            } else {
                ""
            };
            format!("エネルギーを{}払う{}", c.unwrap_or(1), opt)
        }
        "look_and_select" => {
            let look_count = effect.compound.look_action.as_ref().and_then(|a| a.count);
            let select_count = effect.compound.select_action.as_ref().and_then(|a| a.count);
            let select_dest = effect
                .compound
                .select_action
                .as_ref()
                .and_then(|a| a.destination.as_deref())
                .map(|s| zone_label_ja(Some(s)));
            if let (Some(lc), Some(sc)) = (look_count, select_count) {
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
            } else {
                "カードを見て選ぶ".to_string()
            }
        }
        "restriction" => {
            let rt_binding = effect.restriction_type_any();
            let rt = rt_binding.unwrap_or("制限");
            format!("{}制限を適用", rt)
        }
        "gain_ability" => {
            if let Some(ref ga) = effect.ability_gain_any() {
                format!("アビリティを得る：{}", ga)
            } else {
                "アビリティを得る".to_string()
            }
        }
        "do_nothing" => String::new(),
        "sequential" => {
            if let Some(ref actions) = effect.compound.actions {
                let parts: Vec<String> = actions.iter().map(|a| describe_effect_ja(a)).collect();
                if parts.len() == 1 {
                    parts[0].clone()
                } else {
                    format!(
                        "{}、その後{}",
                        parts[..parts.len() - 1].join("、"),
                        parts.last().unwrap()
                    )
                }
            } else {
                effect.text.clone()
            }
        }
        "choice" => "1つを選ぶ".to_string(),
        "play_baton_touch" => {
            let suffix = if c == Some(1) {
                String::new()
            } else {
                format!("（{}体）", c.unwrap_or(1))
            };
            format!("バトンタッチ{}", suffix)
        }
        "modify_required_hearts" | "modify_required_hearts_global" => {
            let val = effect.value_any().unwrap_or(1);
            if val == 0 {
                "必要ハートをクリア".to_string()
            } else if val > 0 {
                format!("必要ハートを{}増やす", val)
            } else {
                format!("必要ハートを{}減らす", (val as i32).unsigned_abs())
            }
        }
        "activate_ability" => {
            if let Some(ref at) = effect.target_trigger_any() {
                format!("{}アビリティを発動", at)
            } else {
                "アビリティを発動".to_string()
            }
        }
        "set_card_identity" => {
            if let Some(ref ids) = effect.identities_any() {
                format!("扱い：{}", ids.join(", "))
            } else {
                "カードの扱いを設定".to_string()
            }
        }
        "reduce_live_card_set_limit" => {
            format!(
                "次のライブカードセットフェイズの上限を{}減らす",
                c.unwrap_or(1)
            )
        }
        "set_blade_count" => format!("ブレードを{}に設定", c.unwrap_or(0)),
        "set_heart_type" => {
            if let Some(ref ht) = effect.heart_type_any() {
                format!("ハートタイプを{}に設定", ht)
            } else {
                "ハートタイプを設定".to_string()
            }
        }
        "set_blade_type" => {
            if let Some(ref bt) = effect.blade_type_any() {
                format!("ブレードタイプを{}に設定", bt)
            } else {
                "ブレードタイプを設定".to_string()
            }
        }
        "discard_until_count" => {
            format!(
                "手札が{}枚になるまで捨てる",
                effect.target_count_any().unwrap_or(1)
            )
        }
        "draw_until_count" => {
            let from = s
                .map(|src| format!("{}から", zone_label_ja(Some(src))))
                .unwrap_or_default();
            format!(
                "手札が{}枚になるまで{}引く",
                effect.target_count_any().unwrap_or(1),
                from
            )
        }
        "modify_cost" => {
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("subtract");
            let amt = c.unwrap_or(1);
            if op == "subtract" {
                format!("{{{{icon_energy.png|E}}}}を{}減らす", amt)
            } else {
                format!("{{{{icon_energy.png|E}}}}を{}増やす", amt)
            }
        }
        "modify_yell_count" => {
            let op_binding = effect.operation_any();
            let op = op_binding.unwrap_or("add");
            let amt = c.unwrap_or(1);
            if op == "subtract" {
                format!("エール回数を{}減らす", amt)
            } else {
                format!("エール回数を{}増やす", amt)
            }
        }
        "perform_yell" => {
            let amt = c.unwrap_or(1);
            let max = effect.repeat_limit_any().unwrap_or(amt);
            if amt == max {
                format!("エールを{}回行う", amt)
            } else {
                format!("最大{}回エールを行う", max)
            }
        }
        "re_yell" => {
            let suffix = if effect.lose_blade_hearts_any().unwrap_or(false) {
                "（ブレード/ハートを失う）"
            } else {
                ""
            };
            format!("再エール{}", suffix)
        }
        "specify_heart_color" => "ハートの色を指定する".to_string(),
        "place_energy_under_member" => {
            let e = effect.energy_count_any().or(c).unwrap_or(1);
            format!("このメンバーの下にエネルギーを{}枚置く", e)
        }
        "invalidate_ability" => "アビリティを無効にする".to_string(),
        "choose_target_player" => "自分か相手を選ぶ".to_string(),
        "repeat_procedure" => {
            let max = effect.repeat_limit_any().unwrap_or(c.unwrap_or(1));
            format!("最大{}回繰り返す", max)
        }
        "sequential_cost" => {
            if let Some(ref costs) = effect.compound.actions {
                let parts: Vec<String> = costs.iter().map(|c| describe_cost_ja(c)).collect();
                let combined = parts.join("\n");
                if costs.last().and_then(|c| c.optional).unwrap_or(false) {
                    format!("{}\n（またはスキップ）", combined)
                } else {
                    combined
                }
            } else {
                effect.text.clone()
            }
        }

        "reveal_until_live_card" => "ライブカードが出るまでデッキを公開する".to_string(),
        "gain_ability_from_source" => "アビリティをコピーする".to_string(),
        "conditional_on_optional" => {
            let parts: Vec<String> = [effect
                .compound
                .conditional_action
                .as_ref()
                .map(|a| describe_effect_ja(a))]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                "コストを払うかスキップ".to_string()
            } else {
                format!("コストを払うかスキップ：{}", parts.join("、"))
            }
        }
        "conditional_on_result" => {
            let parts: Vec<String> = [
                effect
                    .compound
                    .primary_effect
                    .as_ref()
                    .map(|a| describe_effect_ja(a)),
                effect
                    .compound
                    .followup_action
                    .as_ref()
                    .map(|a| describe_effect_ja(a)),
            ]
            .into_iter()
            .flatten()
            .collect();
            if parts.is_empty() {
                effect.text.clone()
            } else {
                parts.join("、その後")
            }
        }
        "conditional_alternative" => {
            let primary = effect
                .compound
                .primary_effect
                .as_ref()
                .map(|a| describe_effect_ja(a));
            let alt = effect
                .alternative_effect_any()
                .as_ref()
                .map(|a| describe_effect_ja(a));
            match (primary, alt) {
                (Some(p), Some(a)) => format!("どちらか：{} / または：{}", p, a),
                (Some(p), None) => p,
                (None, Some(a)) => a,
                (None, None) => effect.text.clone(),
            }
        }
        _ => effect.text.clone(),
    }
}
