use core::sync::atomic::Ordering;

use super::debug::AbDebug;
use crate::ability::debug::ABILITY_DEBUG;
#[cfg(feature = "serde_support")]
use crate::ability::enums::ConditionType;
use crate::ability::enums::Zone;
use crate::ability_queue::ConditionalChoice;
#[cfg(feature = "serde_support")]
use crate::card::CardState;
use crate::card::Condition;
use crate::game_state::Phase;
#[cfg(all(feature = "no_std", feature = "serde_support"))]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "serde_support")]
use serde_json::json;

pub(crate) fn comparison_default_count(condition: &Condition) -> u8 {
    if condition.get_location().is_some() || condition.get_card_type().is_some() {
        1
    } else {
        0
    }
}

pub(crate) fn stage_has_any_member(player: &crate::player::Player) -> bool {
    player.stage.stage.iter().any(|&id| id != -1)
}

/// Read-only context for evaluating ability conditions.
/// Extracted from AbilityResolver to reduce the god-struct surface.
pub struct ConditionContext<'a> {
    pub game_state: &'a crate::game_state::GameState,
    pub activating_card_id: Option<i16>,
    pub moved_cards: &'a [i16],
    pub selected_card_ids: &'a [i16],
    /// Whether a position change occurred this turn (fallback for snapshot).
    pub position_change_occurred: bool,
    /// Cached player reference for "self" target — resolved once at creation.
    self_player: Option<&'a crate::player::Player>,
    /// When true, skip the phase gate check in `check_phase_gate`.
    /// Used by `recalculate_constants` so constant abilities with phase
    /// restrictions (e.g. "during active phase") are always tracked.
    pub skip_phase_gate: bool,
}

impl<'a> ConditionContext<'a> {
    fn resolve_self_player(
        gs: &'a crate::game_state::GameState,
    ) -> Option<&'a crate::player::Player> {
        gs.activating_card.and_then(|cid| {
            if gs.player1.contains_card(cid) {
                Some(&gs.player1)
            } else if gs.player2.contains_card(cid) {
                Some(&gs.player2)
            } else {
                None
            }
        })
    }

    fn build(
        game_state: &'a crate::game_state::GameState,
        moved_cards: &'a [i16],
        selected_card_ids: &'a [i16],
        self_player: Option<&'a crate::player::Player>,
    ) -> Self {
        ConditionContext {
            game_state,
            activating_card_id: game_state.activating_card,
            moved_cards,
            selected_card_ids,
            position_change_occurred: game_state.position_change_occurred_this_turn,
            self_player,
            skip_phase_gate: false,
        }
    }

    pub fn new(game_state: &'a crate::game_state::GameState) -> Self {
        Self::build(game_state, &[], &[], Self::resolve_self_player(game_state))
    }

    pub fn new_with_self(
        game_state: &'a crate::game_state::GameState,
        self_player: Option<&'a crate::player::Player>,
    ) -> Self {
        Self::build(game_state, &[], &[], self_player)
    }

    pub fn with_moved_cards(
        game_state: &'a crate::game_state::GameState,
        moved_cards: &'a [i16],
    ) -> Self {
        Self::build(
            game_state,
            moved_cards,
            &[],
            Self::resolve_self_player(game_state),
        )
    }

    pub fn with_moved_and_selected(
        game_state: &'a crate::game_state::GameState,
        moved_cards: &'a [i16],
        selected_card_ids: &'a [i16],
    ) -> Self {
        Self::build(
            game_state,
            moved_cards,
            selected_card_ids,
            Self::resolve_self_player(game_state),
        )
    }

    /// Returns true if the effect has no condition, or if its condition passes.
    /// Intended as a single-expression guard in sequential and handler loops:
    ///   `if !ConditionContext::new(gs).allows(action) { continue; }`
    /// replaces the repeated 4-line inline check pattern.
    #[inline]
    pub fn allows(&self, effect: &crate::card::AbilityEffect) -> bool {
        effect
            .condition
            .as_ref()
            .map_or(true, |c| self.evaluate_condition(c))
    }
}

/// Produce a user-friendly human-readable expectation string for a condition.
#[cfg(not(feature = "no_std"))]
fn describe_condition_expectation(condition: &Condition) -> String {
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
            if let Some(ref chars) = condition.get_characters() {
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
        | Condition::AllRevealedMatchHeartColor { .. } => String::new(),
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

/// Push a condition verdict to the structured log buffer.
/// `actual_label` overrides the auto-generated actual string; use "" to auto-generate.
#[cfg(not(feature = "no_std"))]
pub fn push_cond_verdict(
    condition: &Condition,
    extra_actual: &str,
    passed: bool,
    children: Vec<crate::ability::log::AbilityLogItem>,
) {
    use crate::ability::log::{push_verdict, AbilityLogItem};
    let condition_type = match condition {
        Condition::Appearance { .. } => "appearance_condition",
        Condition::Comparison { .. } => "comparison_condition",
        Condition::Location { .. } => "card_count_condition",
        Condition::Movement { .. } => "movement_condition",
        Condition::Group { .. } => "group_condition",
        Condition::PositionCond { .. } => "position_condition",
        Condition::Resource { .. } => "resource_condition",
        Condition::ScoreThreshold { .. } => "score_threshold_condition",
        Condition::State { .. } => "state_condition",
        Condition::Temporal { .. } => "temporal_condition",
        Condition::AbilityFilter { .. } => "ability_filter_condition",
        Condition::Choice { .. } => "choice_condition",
        Condition::Complex { .. } => "complex_condition",
        Condition::OpponentChoice { .. } => "opponent_choice_condition",
        Condition::OpponentLiveSuccess { .. } => "opponent_live_success",
        Condition::NoExcessHeart { .. } => "no_excess_heart",
        Condition::Compound { .. } => "compound",
        Condition::AnyOf { .. } => "any_of_condition",
        Condition::AlwaysTrue { .. } => "otherwise_condition",
        Condition::AllRevealedMatchHeartColor { .. } => "all_revealed_match_heart_color",
    }
    .to_string();

    let expectation = describe_condition_expectation(condition);

    let actual = if !extra_actual.is_empty() {
        extra_actual.to_string()
    } else {
        describe_condition_expectation(condition)
    };

    push_verdict(AbilityLogItem::Condition {
        text: describe_condition_expectation(condition),
        condition_type,
        expectation,
        actual,
        passed,
        children,
    });
}

impl<'a> ConditionContext<'a> {
    /// Phase gate: checks whether the condition's phase restriction (if any) is
    /// satisfied.  Handles "自分のメインフェイズ" (self's main phase),
    /// "相手のメインフェイズ" (opponent's main phase), and plain
    /// "メインフェイズ" (any main phase).
    pub fn check_phase_gate(&self, condition: &Condition) -> bool {
        if self.skip_phase_gate {
            return true;
        }
        let te = condition.get_trigger_event();
        let Some(phase) = condition
            .get_phase()
            .or_else(|| te.and_then(|t| t.phase.as_deref()))
        else {
            return true; // no phase restriction
        };
        match phase {
            "main" | "main_phase" => {
                if self.game_state.current_phase != Phase::Main {
                    return false;
                }
                let pt = condition
                    .get_phase_target()
                    .or_else(|| te.and_then(|t| t.phase_target.as_deref()));
                match pt {
                    Some("self") => self
                        .self_player
                        .map(|p| p.id == self.game_state.active_player().id)
                        .unwrap_or(true),
                    Some("opponent") => self
                        .self_player
                        .map(|p| p.id != self.game_state.active_player().id)
                        .unwrap_or(true),
                    _ => true,
                }
            }
            "active_phase" => {
                if self.game_state.current_phase != Phase::Active {
                    return false;
                }
                let pt = condition
                    .get_phase_target()
                    .or_else(|| te.and_then(|t| t.phase_target.as_deref()));
                match pt {
                    Some("self") => self
                        .self_player
                        .map(|p| p.id == self.game_state.active_player().id)
                        .unwrap_or(true),
                    Some("opponent") => self
                        .self_player
                        .map(|p| p.id != self.game_state.active_player().id)
                        .unwrap_or(true),
                    _ => true,
                }
            }
            "live_phase" => {
                if !matches!(
                    self.game_state.current_phase,
                    Phase::LiveCardSetFirstAttacker
                        | Phase::LiveCardSetSecondAttacker
                        | Phase::FirstAttackerPerformance
                        | Phase::SecondAttackerPerformance
                        | Phase::LiveVictoryDetermination
                ) {
                    return false;
                }
                let pt = condition
                    .get_phase_target()
                    .or_else(|| te.and_then(|t| t.phase_target.as_deref()));
                match pt {
                    Some("self") => self
                        .self_player
                        .map(|p| p.id == self.game_state.active_player().id)
                        .unwrap_or(true),
                    Some("opponent") => self
                        .self_player
                        .map(|p| p.id != self.game_state.active_player().id)
                        .unwrap_or(true),
                    _ => true,
                }
            }
            _ => true,
        }
    }

    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        // Handle aggregate total with heart_colors — runs before type dispatch.
        // Skip early return for TemporalCondition so the phase gate is checked too.
        if !matches!(condition, Condition::Temporal { .. })
            && condition.get_aggregate() == Some("total")
            && condition.get_heart_colors().is_some_and(|c| !c.is_empty())
            && Zone::from_str(condition.get_location().unwrap_or("")) != Some(Zone::Stage)
        {
            let location = condition.get_location().unwrap_or("");
            let target = condition.get_target().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            if let Some(result) = self.check_aggregate_total(condition, player, location) {
                return result;
            }
        }

        let mut dbg = AbDebug::new();
        // Snapshot buffer before compound/or so children can be collected
        #[cfg(not(feature = "no_std"))]
        let before = crate::ability::log::buffer_len();
        // Handle compound/or first — they push their own verdicts with children
        if let Condition::Compound { .. } = condition {
            if condition.get_operator() == Some("or") {
                let r = self.evaluate_or_condition(condition);
                return r;
            }
            let r = self.evaluate_compound_condition(condition);
            return r;
        }
        // Phase gate: check phase/phase_target restrictions. This must run
        // BEFORE individual type evaluators so the gate applies universally.
        // Compound/or conditions handle phase internally via sub-condition
        // gates — the top-level gate is skipped for them above.
        if !self.check_phase_gate(condition) {
            return false;
        }
        // For all other types: run evaluator, then push generic verdict
        let result: bool = match condition {
            Condition::Appearance { .. } => self.evaluate_appearance_condition(condition),
            Condition::Comparison { .. } => {
                // both_condition: has values but NO operator and NO comparison_type
                if condition.get_values().is_some() && condition.get_operator().is_none() {
                    self.evaluate_both_condition(condition)
                } else if condition.get_position().is_some()
                    && condition.get_count().is_none()
                    && condition.get_position_compare().is_none()
                    && condition.get_comparison_target().is_none()
                {
                    // highest_cost_on_stage_condition: position set, no count,
                    // no position_compare, and NO comparison_target (if comparison_target
                    // is set, e.g. "opponent", it's a cross-player comparison handled by
                    // evaluate_comparison_condition instead).
                    self.evaluate_highest_cost_on_stage_condition(condition)
                } else if condition.get_comparison_type() == Some("cost")
                    && condition.get_position().is_none()
                    && condition.get_count().is_none()
                    && condition.get_values().is_none()
                    && (condition.get_comparison_target().is_none()
                        || condition.get_comparison_target()
                            == Some(crate::card::ComparisonTarget::Self_))
                    && condition.get_all().unwrap_or(false)
                {
                    // all_cost_comparison_condition: cost comparison, no position/count/values,
                    // comparison_target is either None or Self_, AND all=true
                    self.evaluate_all_cost_comparison_condition(condition)
                } else {
                    self.evaluate_comparison_condition(condition)
                }
            }
            Condition::Location { .. } => {
                if condition.get_count().is_some()
                    || condition.get_source() == Some("preceding_moved")
                {
                    // For distinct conditions with multiple locations (e.g. stage+waitroom),
                    // route to evaluate_multi_location_condition which handles combined zones.
                    if condition.get_distinct().is_some_and(|d| d.is_distinct())
                        && condition.get_locations().is_some()
                    {
                        self.evaluate_multi_location_condition(condition)
                    } else {
                        self.evaluate_card_count_condition(condition)
                    }
                } else {
                    self.evaluate_location_condition(condition)
                }
            }
            Condition::Resource { .. } => {
                if condition.get_resource_type().is_some() {
                    self.evaluate_resource_condition(condition)
                } else {
                    self.evaluate_card_blade_condition(condition)
                }
            }
            Condition::Group { .. } => self.evaluate_group_condition(condition),
            Condition::PositionCond { .. } => self.evaluate_position_condition(condition),
            Condition::Temporal { .. } => self.evaluate_temporal_condition(condition),
            Condition::Movement { .. } => self.evaluate_movement_condition(condition),
            Condition::State { energy_state, .. } => {
                if condition.get_from_state().is_some() || condition.get_to_state().is_some() {
                    self.evaluate_state_change_condition(condition)
                } else if energy_state.is_some() {
                    self.evaluate_energy_state_condition(condition)
                } else {
                    self.evaluate_state_condition(condition)
                }
            }
            Condition::AbilityFilter { .. } => self.evaluate_ability_filter_condition(condition),
            Condition::AnyOf { .. } => self.evaluate_any_of_condition(condition),
            Condition::ScoreThreshold { .. } => self.evaluate_score_threshold_condition(condition),
            Condition::Choice { .. } => self.evaluate_choice_condition(condition),
            Condition::OpponentChoice { .. } => self.evaluate_opponent_choice_condition(condition),
            Condition::OpponentLiveSuccess { .. } => {
                self.evaluate_opponent_live_success_condition(condition)
            }
            Condition::Complex { .. } => self.evaluate_complex_condition(condition),
            Condition::NoExcessHeart { .. } => self.evaluate_no_excess_heart_condition(condition),
            Condition::AlwaysTrue { .. } => true,
            Condition::AllRevealedMatchHeartColor { .. } => {
                self.evaluate_all_revealed_match_heart_color(condition)
            }
            Condition::Compound { .. } => unreachable!(),
        };

        let is_plain_location = matches!(condition, Condition::Location { .. })
            && condition.get_count().is_none();

        let final_result = if condition.get_negation().unwrap_or(false)
            && !(matches!(condition, Condition::Location { .. })
                && condition.get_card_property().is_some())
            && !(matches!(condition, Condition::Movement { .. })
                && condition.get_card_property().is_some())
            && !(is_plain_location && condition.get_heart_type() == Some("all"))
            && !(is_plain_location
                && condition.get_location() == Some("revealed_cards")
                && self.game_state.revealed_cards.is_empty())
        {
            !result
        } else {
            result
        };
        // Push ONE verdict per condition with actual game state value.
        // Skip if the sub-type-specific evaluator already pushed a verdict
        // (e.g. comparison_condition, card_count_condition).
        #[cfg(not(feature = "no_std"))]
        {
            if crate::ability::log::buffer_len() <= before {
                let actual = self.describe_condition_actual(condition);
                push_cond_verdict(condition, &actual, final_result, vec![]);
            }
        }
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
            let thresh = if matches!(condition, Condition::Comparison { .. }) {
                condition.get_count().unwrap_or(0)
            } else {
                1
            };
            let dbg_actual = if result {
                condition.get_count().unwrap_or(1)
            } else {
                0
            };
            dbg.condition(condition, dbg_actual, thresh, final_result);
        }

        if let Some(filter) = condition.get_ability_filter() {
            // MovementCondition handles ability_filter internally (applies to
            // the baton-touch source/replaced member, not the activating card).
            if !matches!(condition, Condition::Movement { .. }) {
                let filtered =
                    self.evaluate_ability_filter_condition_with_card_check(condition, filter);
                if !filtered {
                    return false;
                }
            }
        }

        final_result
    }

    /// Evaluates whether all cards in the revealed zone match the specified heart color.
    /// Reads the chosen heart color from conditional_choice (set by specify_heart_color action).
    /// Member cards must have the color in base_heart; live cards must have it in need_heart.
    pub fn evaluate_all_revealed_match_heart_color(&self, condition: &Condition) -> bool {
        let chosen_color = self.game_state.ability_queue.current_entry().and_then(|e| {
            match &e.conditional_choice {
                Some(ConditionalChoice::Str(s)) => Some(s.clone()),
                _ => None,
            }
        });
        let color = match chosen_color {
            Some(ref c) => c.clone(),
            None => {
                if let Some(cid) = self.activating_card_id {
                    if let Some(&override_color) =
                        self.game_state.mods.heart_color_multiplier.get(&cid)
                    {
                        format!("{}", override_color)
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        };
        let cards = &self.game_state.revealed_cards;
        let count = condition.get_count().unwrap_or(1) as usize;
        let operator = condition.get_operator().unwrap_or(">=");
        let card_db = &self.game_state.card_database;

        let matching = cards
            .iter()
            .filter(|&&cid| {
                crate::ability::util::card_matches_heart_colors(card_db, cid, &[color.clone()])
            })
            .count();

        match operator {
            ">=" => matching >= count,
            ">" => matching > count,
            "=" => matching == count,
            "<=" => matching <= count,
            "<" => matching < count,
            _ => matching >= count,
        }
    }

    /// Evaluate condition and return structured actual value for debug display.
    /// Same as evaluate_condition but also returns the measured runtime value
    /// (count, score, bool, etc.) that was compared against the condition threshold.
    /// Used by the /api/debug/conditions endpoint — purely read-only.
    #[cfg(feature = "serde_support")]
    pub fn evaluate_condition_debug(&self, condition: &Condition) -> (bool, serde_json::Value) {
        let passed = self.evaluate_condition(condition);
        let actual_str = self.describe_condition_actual(condition);
        let count = self.get_count_for_condition(condition);
        let ct = condition.condition_type();
        let threshold = match ct {
            Some(ConditionType::ComparisonCondition) => condition.get_count().unwrap_or(0),
            Some(ConditionType::ScoreThresholdCondition) => condition.get_count().unwrap_or(0),
            Some(ConditionType::CardCountCondition) => condition.get_count().unwrap_or(1),
            Some(ConditionType::CardBladeCondition) => condition.get_count().unwrap_or(1),
            _ => condition.get_count().unwrap_or(0),
        };
        (
            passed,
            json!({
                "measure": actual_str,
                "threshold": threshold,
                "count": count,
            }),
        )
    }

    /// Query game state to produce a human-readable "actual" string for this condition.
    /// This runs immediately after evaluation (game state is fresh).
    #[cfg(feature = "serde_support")]
    fn describe_condition_actual(&self, condition: &Condition) -> String {
        let ct = condition.condition_type();
        match ct {
            Some(ConditionType::AppearanceCondition) => self.describe_appearance_actual(condition),
            Some(ConditionType::ComparisonCondition) => {
                let count = self.get_count_for_condition(condition);
                format!("{}", count)
            }
            Some(ConditionType::BothCondition) => {
                let count = self.get_count_for_condition(condition);
                let vals = condition
                    .get_values()
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default();
                format!("count={}, values={}", count, vals)
            }
            Some(ConditionType::CardCountCondition) => {
                let count = self.get_count_for_condition(condition);
                format!("{}", count)
            }
            Some(ConditionType::CardBladeCondition) => {
                if let Some(op) = condition.get_operator() {
                    format!(
                        "{} {} {}",
                        "ブレード",
                        op,
                        condition.get_count().unwrap_or(1)
                    )
                } else {
                    String::new()
                }
            }
            Some(ConditionType::GroupCondition) => {
                let player =
                    self.resolve_condition_player(condition.get_target().unwrap_or("self"));
                let loc = condition.get_location().unwrap_or("stage");
                let ids: Vec<i16> = match Zone::from_str(loc) {
                    Some(Zone::Stage) => player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect(),
                    _ => vec![],
                };
                let names: Vec<String> = ids
                    .iter()
                    .filter_map(|&cid| {
                        self.game_state
                            .card_database
                            .get_card(cid)
                            .map(|c| c.name.to_string())
                    })
                    .collect();
                if names.is_empty() {
                    "不在".into()
                } else {
                    format!("在籍=[{}]", names.join(","))
                }
            }
            Some(ConditionType::PositionCondition) => {
                let player =
                    self.resolve_condition_player(condition.get_target().unwrap_or("self"));
                let ids: Vec<(usize, &i16)> = player
                    .stage
                    .stage
                    .iter()
                    .enumerate()
                    .filter(|(_, &id)| id != -1)
                    .collect();
                if ids.is_empty() {
                    "不在".into()
                } else {
                    let pos_names = ["左", "中", "右"];
                    let desc: Vec<String> = ids
                        .iter()
                        .map(|(i, &id)| {
                            let name = self
                                .game_state
                                .card_database
                                .get_card(id)
                                .map(|c| c.name.to_string())
                                .unwrap_or_default();
                            format!("{}:{}", pos_names[*i], name)
                        })
                        .collect();
                    desc.join(", ")
                }
            }
            Some(ConditionType::LocationCondition) => {
                let loc = condition.get_location().unwrap_or("");
                if let Some(ref pos) = condition.get_position() {
                    let pos_str = pos.get_position().unwrap_or("?");
                    format!("位置={}", pos_str)
                } else {
                    format!("{}", loc)
                }
            }
            Some(ConditionType::StateCondition) => {
                let state = condition.get_state().map(|s| s.as_str()).unwrap_or("状態");
                let target = condition.get_target().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                let resource_type = condition.get_resource_type();
                if resource_type == Some("energy") {
                    let active = player.energy_zone.active_count();
                    let total = player.energy_zone.cards.len();
                    format!("エネルギー active={}/{}", active, total)
                } else {
                    let loc = condition.get_location().unwrap_or("stage");
                    let stage_cards: Vec<i16> = match Zone::from_str(loc) {
                        Some(Zone::Stage) => player
                            .stage
                            .stage
                            .iter()
                            .filter(|&&id| id != -1)
                            .copied()
                            .collect(),
                        _ => vec![],
                    };
                    let matching = stage_cards
                        .iter()
                        .filter(|&&cid| {
                            crate::ability::util::orientation_matches_state(
                                self.game_state.mods.get_orientation_modifier(cid),
                                state,
                            )
                        })
                        .count();
                    format!(
                        "{}状態のメンバー={}枚 (ステージ計{}枚)",
                        state,
                        matching,
                        stage_cards.len()
                    )
                }
            }
            Some(ConditionType::MovementCondition) => {
                let mov = condition.get_movement().unwrap_or("?");
                let count = self
                    .game_state
                    .recently_moved_cards
                    .as_ref()
                    .map(|v| v.len())
                    .unwrap_or(0);
                format!("移動={}, 移動枚数={}", mov, count)
            }
            Some(ConditionType::TemporalCondition) => {
                let appeared = self.game_state.cards_appeared_this_turn.len();
                let moved = self.game_state.cards_moved_this_turn.len();
                format!("登場={}, 移動={}", appeared, moved)
            }
            Some(ConditionType::NoExcessHeart) => {
                if self.no_excess_heart_flag(condition.get_target().unwrap_or("self")) {
                    "余剰ハートなし".into()
                } else {
                    "余剰ハートあり".into()
                }
            }
            Some(ConditionType::AnyOfCondition) => {
                if let Some(ref any_of) = condition.get_any_of() {
                    format!("条件={:?}", any_of)
                } else {
                    String::new()
                }
            }
            Some(ConditionType::ChoiceCondition) => {
                if let Some(ref opts) = condition.get_options() {
                    format!("選択肢={}個", opts.len())
                } else {
                    "選択肢なし".into()
                }
            }
            Some(ConditionType::EnergyStateCondition) => condition
                .get_state()
                .map(CardState::as_str)
                .map(|s| format!("エネルギー状態={}", s))
                .unwrap_or_default(),
            Some(ConditionType::StateChangeCondition) => {
                let from = condition.get_from_state().unwrap_or("?");
                let to = condition.get_to_state().unwrap_or("?");
                format!("状態変化: {}→{}", from, to)
            }
            Some(ConditionType::AllCostComparisonCondition) => {
                let op = condition.get_operator().unwrap_or(">");
                format!("全コスト比較{}?", op)
            }
            Some(ConditionType::ScoreThresholdCondition) => {
                let op = condition.get_operator().unwrap_or(">=");
                format!("スコア{} {}?", op, condition.get_count().unwrap_or(1))
            }
            Some(ConditionType::ResourceCondition) => {
                format!("資源={}", condition.get_resource_type().unwrap_or("?"))
            }
            _ => String::new(),
        }
    }

    #[cfg(feature = "serde_support")]
    fn describe_appearance_actual(&self, condition: &Condition) -> String {
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let location = condition.get_location().unwrap_or("");

        // Check position constraints first
        let mut position_str = String::new();
        if let Some(ref pos) = condition.get_position() {
            position_str = format!("位置={}", pos.get_position().unwrap_or("?"));
        } else if let Some(ref act_pos) = condition.get_activation_position() {
            let card_id = self.activating_card_id;
            let ok = act_pos.split(',').any(|p| {
                let trimmed = p.trim();
                let idx = match trimmed {
                    "left" | "left_side" => 0,
                    "center" => 1,
                    "right" | "right_side" => 2,
                    _ => return true,
                };
                idx < player.stage.stage.len()
                    && card_id.is_some()
                    && player.stage.stage[idx] == card_id.unwrap()
            });
            if ok {
                position_str = format!("位置=OK({})", act_pos);
            } else {
                let actual_pos = card_id
                    .and_then(|id| {
                        player
                            .stage
                            .stage
                            .iter()
                            .position(|&c| c == id)
                            .map(|i| ["左", "中", "右"][i])
                    })
                    .unwrap_or("?");
                position_str = format!("位置=不適合(現在{}、期待{})", actual_pos, act_pos);
            }
        }

        match Zone::from_str(location) {
            Some(Zone::Stage) => {
                let stage_ids: Vec<i16> = player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .copied()
                    .collect();
                if stage_ids.is_empty() {
                    return format!("不在 {}", position_str).trim().to_string();
                }
                let stage_names: Vec<String> = stage_ids
                    .iter()
                    .filter_map(|&cid| {
                        self.game_state
                            .card_database
                            .get_card(cid)
                            .map(|c| crate::card::CardDatabase::normalize_name(&c.name))
                    })
                    .collect();
                // Check character match
                if let Some(chars) = condition.get_characters() {
                    for ch in chars {
                        let norm = crate::card::CardDatabase::normalize_name(ch);
                        let found = stage_names.iter().any(|n| n.contains(&norm));
                        if !found {
                            return format!("{}不在 {}", ch, position_str).trim().to_string();
                        }
                    }
                    // All matched — check cost_reference
                    if let Some(ref ref_char) = condition.get_cost_reference_character() {
                        let subject = &chars[0];
                        let norm_sub = crate::card::CardDatabase::normalize_name(subject);
                        let norm_ref = crate::card::CardDatabase::normalize_name(ref_char);
                        let sub_cost = stage_ids
                            .iter()
                            .filter_map(|&cid| {
                                let card = self.game_state.card_database.get_card(cid)?;
                                let n = crate::card::CardDatabase::normalize_name(&card.name);
                                if n.contains(&norm_sub) {
                                    card.cost
                                } else {
                                    None
                                }
                            })
                            .next();
                        let ref_cost = stage_ids
                            .iter()
                            .filter_map(|&cid| {
                                let card = self.game_state.card_database.get_card(cid)?;
                                let n = crate::card::CardDatabase::normalize_name(&card.name);
                                if n.contains(&norm_ref) {
                                    card.cost
                                } else {
                                    None
                                }
                            })
                            .next();
                        let op = condition
                            .get_cost_reference_operator()
                            .map(|o| o.as_str())
                            .unwrap_or(">");
                        let cost_part = match (sub_cost, ref_cost) {
                            (Some(sc), Some(rc)) => format!(
                                "{}コスト({}) {} {}コスト({})",
                                subject, sc, op, ref_char, rc
                            ),
                            (Some(sc), None) => {
                                format!("{}コスト({}) {} {} (不在)", subject, sc, op, ref_char)
                            }
                            (None, Some(rc)) => {
                                format!("{}(不在) {} {}コスト({})", subject, op, ref_char, rc)
                            }
                            (None, None) => format!("{}も{}も不在", subject, ref_char),
                        };
                        if position_str.is_empty() {
                            cost_part
                        } else {
                            format!("{} {}", cost_part, position_str)
                        }
                    } else {
                        let names: Vec<String> = stage_ids
                            .iter()
                            .filter_map(|&cid| {
                                self.game_state
                                    .card_database
                                    .get_card(cid)
                                    .map(|c| c.name.to_string())
                            })
                            .collect();
                        let base = format!("在籍=[{}]", names.join(", "));
                        if position_str.is_empty() {
                            base
                        } else {
                            format!("{} {}", base, position_str)
                        }
                    }
                } else {
                    let names: Vec<String> = stage_ids
                        .iter()
                        .filter_map(|&cid| {
                            self.game_state
                                .card_database
                                .get_card(cid)
                                .map(|c| c.name.to_string())
                        })
                        .collect();
                    let base = format!("在籍=[{}]", names.join(", "));
                    if position_str.is_empty() {
                        base
                    } else {
                        format!("{} {}", base, position_str)
                    }
                }
            }
            Some(Zone::Hand) => format!("手札={}枚", player.hand.cards.len()),
            Some(Zone::Discard) => format!("控え室={}枚", player.waitroom.cards.len()),
            _ => String::new(),
        }
    }
}

mod card;
mod compound;
mod state;
