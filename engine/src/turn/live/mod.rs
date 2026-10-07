use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::enums::Zone;
use crate::card::{BaseHeart, BladeColor, CardDatabase, HeartColor};
use crate::core::game_modifiers::ModifierEntry;
use crate::game_state::GameState;
use crate::turn::TurnEngine;
use crate::types::{
    AdjustmentType, AllocPhase, Allocation, ArcStr, BladeSource, HeartSource, LivePerformanceData,
    MemberContribution, SourceName, SourceType, YellCardResult,
};
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::Ordering;

use scoring::EMPTY_H8;
// ============== SNAPSHOT BUILDING ==============

/// Process the ability_applications recorded during the live performance
/// and populate MemberContribution ability bonuses, score lines, etc.
pub fn enrich_from_applications(
    member_contributions: &mut [MemberContribution],
    breakdown: &mut crate::types::Breakdown,
    triggered_abilities: &mut Vec<crate::types::TriggeredAbility>,
    applications: &[crate::types::AbilityApplication],
    card_db: &crate::card::CardDatabase,
) {
    let mut seen = HashSet::<(i16, &ArcStr)>::default();
    for app in applications {
        if let Some(mc) = member_contributions
            .iter_mut()
            .find(|m| m.source_id == app.target_card_id)
        {
            match app.effect_type {
                crate::types::EffectType::HeartBonus => {
                    mc.ability_heart_bonuses.push(crate::types::AbilityBonus {
                        source: bonus_source(card_db, app.source_card_id),
                        amount: u8::try_from(app.amount.unsigned_abs()).unwrap(),
                        color: app.heart_color,
                        ability_text: app.ability_text.clone(),
                    });
                }
                crate::types::EffectType::BladeBonus => {
                    mc.ability_blade_bonuses.push(crate::types::AbilityBonus {
                        source: bonus_source(card_db, app.source_card_id),
                        amount: u8::try_from(app.amount.unsigned_abs()).unwrap(),
                        color: app.heart_color,
                        ability_text: app.ability_text.clone(),
                    });
                }
                _ => {}
            }
        }
        match app.effect_type {
            crate::types::EffectType::ScoreBonus | crate::types::EffectType::ScoreSet => {
                breakdown.scores.push(crate::types::ScoreLine {
                    source: debug_label(|| app.ability_text.to_string()),
                    value: u8::try_from(app.amount.unsigned_abs()).unwrap(),
                });
            }
            crate::types::EffectType::Transform => {
                breakdown.transforms.push(crate::types::EffectEntry {
                    source: debug_label(|| app.ability_text.to_string()),
                    desc: debug_label(|| {
                        format!("All hearts become type {}", app.heart_color.unwrap_or(0))
                    }),
                    value: String::new(),
                });
            }
            _ => {}
        }
        let key = (app.source_card_id, &app.ability_text);
        if seen.insert(key) && !app.ability_text.is_empty() {
            let card = card_db.get_card(app.source_card_id);
            triggered_abilities.push(crate::types::TriggeredAbility {
                source_card_id: app.source_card_id,
                name: debug_label(|| format!("Ability #{}", triggered_abilities.len() + 1)),
                card_name: card
                    .map(|c| crate::types::ArcStr::from(c.name.as_ref()))
                    .unwrap_or_default(),
                effect_text: app.ability_text.clone(),
                condition_text: None,
                is_public: true,
            });
        }
    }
}

/// Compute a debug-only label, or the empty default when ABILITY_DEBUG is off.
///
/// These labels exist for the live breakdown log and nothing else. They are
/// built from `make` lazily so the formatting cost is not paid on the normal
/// path, and so every call site spells the flag check the same way rather than
/// each one re-deriving "is the debug build on?".
pub(crate) fn debug_label(make: impl FnOnce() -> String) -> String {
    if ABILITY_DEBUG.load(Ordering::Relaxed) {
        make()
    } else {
        String::new()
    }
}

/// The `Ability: <name>` source label for a recorded bonus, or empty when
/// ABILITY_DEBUG is off.
pub(crate) fn bonus_source(card_db: &CardDatabase, source_card_id: i16) -> crate::types::ArcStr {
    if !ABILITY_DEBUG.load(Ordering::Relaxed) {
        return crate::types::ArcStr::default();
    }
    let name = card_db
        .get_card(source_card_id)
        .map(|c| c.name.to_string())
        .unwrap_or_else(|| format!("#{}", source_card_id));
    format!("Ability: {}", name).into()
}

#[allow(clippy::too_many_arguments)]
pub fn build_snapshot(
    turn: u8,
    player_id: &str,
    perf: LivePerformanceData,
    card_db: &CardDatabase,
    note_icons: u8,
    performance_need_heart_modifiers: &[(i16, HeartColor, ModifierEntry)],
    member_contributions: Vec<crate::types::MemberContribution>,
    breakdown: crate::types::Breakdown,
    triggered_abilities: Vec<crate::types::TriggeredAbility>,
) -> crate::types::PerformanceSnapshot {
    let mut lives = Vec::new();
    // Use perf.live_card_ids (captured before heart check cleared the zone)
    for &lc_id in &perf.live_card_ids {
        let score = card_db.get_card(lc_id).map(|c| c.get_score()).unwrap_or(0);
        let card_no = card_db
            .get_card(lc_id)
            .map(|c| crate::types::ArcStr::from(c.card_no.as_ref()))
            .unwrap_or_default();
        lives.push(crate::types::LiveCardResult {
            passed: false,
            score,
            base_score: score,
            spare: EMPTY_H8,
            required: EMPTY_H8,
            filled: EMPTY_H8,
            adjustments: Vec::new(),
            card_id: lc_id,
            card_no,
        });
    }

    crate::types::PerformanceSnapshot {
        turn,
        player_id: player_id.to_string(),
        lives,
        member_contributions,
        yell_cards: perf.yell_cards,
        total_hearts: perf.total_hearts,
        total_score: 0,
        success: false,
        note_icons,
        yell_count: perf.yell_count,
        breakdown,
        triggered_abilities,
        surplus_hearts: [0; 8],
        revealed_ids: perf.revealed_ids,
        p0_wins: false,
        p1_wins: false,
        performance_need_heart_modifiers: performance_need_heart_modifiers.to_vec(),
        base_score_total: 0,
        card_bonus_total: 0,
    }
}

pub(crate) fn fmt_player_id(id: &str) -> String {
    let mut digits = id.chars().filter(|c| c.is_ascii_digit());
    match digits.next() {
        None => "?".to_string(),
        Some(first) => {
            let mut s = String::with_capacity(8);
            s.push('P');
            s.push(first);
            s.extend(digits);
            s
        }
    }
}

mod scoring;
mod cheer;
mod resolution;

pub use scoring::{blade_color_to_heart, blade_type_override_for_stage};
pub use cheer::{process_yell_revealed_card_icons, YellIconOutcome};
