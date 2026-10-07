//! `describe_cost` / `describe_sequential_cost` and their `_en`/`_ja` wrappers.
//!
//! A cost is an `AbilityEffect` whose `action` is one of the cost-bearing
//! `ActionType`s (`PayEnergy`, `ChangeState`, `MoveCards`, `Reveal`); the four
//! `_en`/`_ja` wrappers are thin locale selectors over the two parameterised
//! functions, kept here rather than inlined so callers can pass a literal.
//!
//! `describe_cost` delegates to `effect::describe_effect` for the fallback arm,
//! so this file depends on `effect.rs` — one direction only, no cycle.

use super::*;

use crate::ability::enums::{ActionType, Zone};
use crate::card::AbilityEffect;
use super::effect::describe_effect;
#[cfg(feature = "no_std")]
use alloc::boxed::Box;


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
