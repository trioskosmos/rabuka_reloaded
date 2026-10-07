//! Condition expectation renderer.
//!
//! `describe_condition_expectation` produces the human-readable string shown in
//! a condition verdict. It used to live in `ability/condition/mod.rs` beside the
//! evaluator that consumes it, which put the rendering of a *condition* in the
//! same file as the *condition* type's dispatch logic — two jobs with one
//! dependency direction (the evaluator never calls back into the renderer).
//!
//! The two private zone/card-type label helpers stay here because they are
//! only ever reached from this file.

use crate::card::Condition;

/// Produce a user-friendly human-readable expectation string for a condition.
#[cfg(not(feature = "no_std"))]
pub fn describe_condition_expectation(condition: &Condition) -> String {
    let op = condition.get_operator().unwrap_or(">=");
    let threshold = condition
        .get_count()
        .map(|c| c.to_string())
        .unwrap_or_default();
    let location = condition.get_location().unwrap_or("");
    let ct_field = condition
        .get_card_type()
        .map(|ct| ct.as_str())
        .unwrap_or("");

    match condition {
        Condition::Appearance { .. } => {
            if let Some(chars) = condition.get_characters() {
                if !chars.is_empty() {
                    let has_cost_ref = matches!(
                        condition,
                        Condition::Appearance {
                            cost_reference_character: Some(_),
                            ..
                        }
                    );
                    if has_cost_ref {
                        if let Condition::Appearance {
                            cost_reference_operator,
                            cost_reference_character,
                            ..
                        } = condition
                        {
                            format!(
                                "{} {} {}",
                                chars[0],
                                cost_reference_operator.as_deref().unwrap_or(">"),
                                cost_reference_character.as_deref().unwrap_or("")
                            )
                        } else {
                            format!("{} = true", chars[0])
                        }
                    } else {
                        format!("{} = true", chars[0])
                    }
                } else {
                    "登場=true".into()
                }
            } else {
                "登場=true".into()
            }
        }
        Condition::Comparison { .. } => {
            let loc = if !location.is_empty() {
                format!(" ({})", describe_zone_label(location))
            } else {
                String::new()
            };
            format!("{} {}{}", op, threshold, loc)
        }
        Condition::Location { .. } => {
            let zone_desc = if !location.is_empty() {
                describe_zone_label(location)
            } else {
                "anywhere".into()
            };
            let type_desc = if !ct_field.is_empty() {
                describe_card_type_label(ct_field)
            } else {
                "cards".into()
            };
            format!("{} {} {} in {}", op, threshold, type_desc, zone_desc)
        }
        Condition::Group { .. } => {
            if let Some(gns) = condition.get_group_names() {
                format!("所属={}", gns.join(","))
            } else {
                "所属条件".into()
            }
        }
        Condition::PositionCond { .. } => {
            if let Some(pos) = condition.get_position() {
                format!("位置={}", pos.get_position().unwrap_or("?"))
            } else {
                "位置条件".into()
            }
        }
        Condition::Resource { .. } => {
            format!("ブレード {} {}", op, threshold)
        }
        Condition::ScoreThreshold { .. } => {
            format!("スコア {} {}", op, threshold)
        }
        Condition::State { .. } => {
            let st = condition.get_state().map(|s| s.as_str()).unwrap_or("状態");
            let loc = describe_zone_label(location);
            format!("{}状態のメンバー in {}", st, loc)
        }
        Condition::Movement { .. } => {
            format!("移動={}", condition.get_movement().unwrap_or("?"))
        }
        Condition::Temporal { .. } => condition.get_temporal().unwrap_or("タイミング").to_string(),
        Condition::AbilityFilter { .. } => "フィルター".into(),
        Condition::NoExcessHeart { .. } => "余剰ハートなし".into(),
        Condition::Choice { .. }
        | Condition::Complex { .. }
        | Condition::OpponentChoice { .. }
        | Condition::OpponentLiveSuccess { .. }
        | Condition::Compound { .. }
        | Condition::AnyOf { .. }
        | Condition::AlwaysTrue { .. }
        | Condition::AllRevealedMatchHeartColor { .. }
        | Condition::Unsupported { .. } => String::new(),
    }
}

#[cfg(not(feature = "no_std"))]
fn describe_zone_label(zone: &str) -> String {
    match zone {
        "hand" => "[[zone_hand]]".into(),
        "discard" | "waitroom" => "[[zone_discard]]".into(),
        "deck" => "[[zone_deck]]".into(),
        "deck_top" => "[[zone_deck_top]]".into(),
        "deck_bottom" => "[[zone_deck_bottom]]".into(),
        "stage" => "[[zone_stage]]".into(),
        "energy" | "energy_zone" => "[[zone_energy]]".into(),
        "live_card_zone" => "[[zone_live_card]]".into(),
        "success_zone" | "success_live_zone" => "[[zone_success_live]]".into(),
        "revealed_cards" => "[[zone_revealed]]".into(),
        "those_cards" => "[[zone_those_cards]]".into(),
        "all_selected" => "[[zone_selected]]".into(),
        "under_member" => "[[zone_under_member]]".into(),
        _ => zone.to_string(),
    }
}

#[cfg(not(feature = "no_std"))]
fn describe_card_type_label(ct: &str) -> String {
    match ct {
        "member_card" => "[[card_type_member]]".into(),
        "live_card" => "[[card_type_live]]".into(),
        "energy_card" => "[[card_type_energy]]".into(),
        "card" => "[[card_type_card]]".into(),
        _ => ct.to_string(),
    }
}