use crate::card::AbilityEffect;

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

fn zone_label(zone: Option<&str>) -> &str {
    match zone {
        Some("hand") => "hand",
        Some("discard") => "the waiting room",
        Some("deck") => "deck",
        Some("deck_top") => "top of deck",
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

fn card_type_label(ct: Option<&str>) -> &str {
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
    let ct = card_type_label(effect.card_type.as_deref());
    let c = effect.count;
    let t = effect.target.as_deref();
    let s = effect.source.as_deref();
    let d = effect.destination.as_deref();
    let gn = group_label(effect.group_names.as_ref());

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
                    if let Some("wait") = effect.state_change.as_deref() {
                        result += " (rest)";
                    }
                    result
                }
            }
        }
        "draw_card" => maybe_plural(c, "card"),

        "gain_resource" => {
            let r = resource_label(effect.resource.as_deref());
            let dur = effect.duration.as_deref().and_then(|d| {
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
            let verb = state_verb(effect.state_change.as_deref());
            let cnt = c.unwrap_or(1);
            let who = match t {
                Some("opponent") => "opponent ",
                _ => "",
            };
            let lim = effect
                .cost_limit
                .map(|cl| format!(" (cost ≤ {})", cl))
                .unwrap_or_default();
            format!("{} {}{} {}{}", verb, cnt, gn, who, lim)
        }

        "modify_score" => {
            let val = effect.value.unwrap_or(1);
            let op = effect.operation.as_deref().unwrap_or("add");
            if op == "subtract" {
                format!("Subtract {} from score", val)
            } else {
                format!("Add {} to score", val)
            }
        }

        "position_change" => {
            if let Some(ep) = effect.exclude_position.as_deref() {
                format!("Move a{} member away from {}", gn, ep)
            } else if c == Some(1) || c.is_none() {
                format!("Position change a{} member", gn)
            } else {
                format!("Position change {}{} members", c.unwrap_or(1), gn)
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
            let rt = effect.restriction_type.as_deref().unwrap_or("restriction");
            format!("Apply {} restriction", rt)
        }

        "gain_ability" => {
            if let Some(ref ga) = effect.ability_gain {
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

        "choice" => {
            if let Some(ref opts) = effect.options {
                let parts: Vec<String> = opts.iter().map(|o| describe_effect_en(o)).collect();
                format!("Choose 1:\n{}", parts.join("\n"))
            } else {
                "Choose 1".to_string()
            }
        }

        "play_baton_touch" => format!(
            "Baton touch {} member{}",
            c.unwrap_or(1),
            if c == Some(1) { "" } else { "s" }
        ),

        "modify_required_hearts" | "modify_required_hearts_global" => {
            let val = effect.value.unwrap_or(1);
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
            if let Some(ref at) = effect.target_trigger {
                format!("Activate {} ability", at)
            } else {
                "Activate ability".to_string()
            }
        }

        "set_card_identity" => {
            if let Some(ref ids) = effect.identities {
                format!("Treat as: {}", ids.join(", "))
            } else {
                "Set card identity".to_string()
            }
        }

        "set_blade_count" => format!("Set blade to {}", c.unwrap_or(0)),

        "set_heart_type" => {
            if let Some(ref ht) = effect.heart_type {
                format!("Set heart type to {}", ht)
            } else {
                "Set heart type".to_string()
            }
        }

        "set_blade_type" => {
            if let Some(ref bt) = effect.blade_type {
                format!("Set blade type to {}", bt)
            } else {
                "Set blade type".to_string()
            }
        }

        "discard_until_count" => {
            format!(
                "Discard down to {} card{} in hand",
                effect.target_count.unwrap_or(1),
                if effect.target_count == Some(1) {
                    ""
                } else {
                    "s"
                }
            )
        }

        "draw_until_count" => {
            format!(
                "Draw until {} card{} in hand",
                effect.target_count.unwrap_or(1),
                if effect.target_count == Some(1) {
                    ""
                } else {
                    "s"
                }
            )
        }

        "modify_cost" => {
            let op = effect.operation.as_deref().unwrap_or("subtract");
            let amt = c.unwrap_or(1);
            if op == "subtract" {
                format!("Reduce cost by {}", amt)
            } else {
                format!("Increase cost by {}", amt)
            }
        }

        "modify_yell_count" => {
            let op = effect.operation.as_deref().unwrap_or("add");
            let amt = c.unwrap_or(1);
            if op == "subtract" {
                format!("Reduce yell count by {}", amt)
            } else {
                format!("Increase yell count by {}", amt)
            }
        }

        "perform_yell" => {
            let amt = c.unwrap_or(1);
            let max = effect.repeat_limit.unwrap_or(amt);
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
                if effect.lose_blade_hearts.unwrap_or(false) {
                    " (lose blade/hearts)"
                } else {
                    ""
                }
            )
        }

        "specify_heart_color" => "Choose a heart color".to_string(),

        "place_energy_under_member" => {
            let e = effect.energy_count.or(c).unwrap_or(1);
            format!("Place {} energy under this member", e)
        }

        "invalidate_ability" => "Cancel an ability".to_string(),

        "choose_target_player" => "Choose self or opponent".to_string(),

        "repeat_procedure" => {
            let max = effect.repeat_limit.unwrap_or(c.unwrap_or(1));
            format!("Repeat up to {} times", max)
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
                .alternative_effect
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
