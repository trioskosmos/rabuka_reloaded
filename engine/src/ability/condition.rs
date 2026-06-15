use super::debug::AbDebug;
use crate::ability::enums::ConditionType;
use crate::ability::enums::Zone;
use crate::card::Condition;

pub(crate) fn comparison_default_count(condition: &Condition) -> u32 {
    if condition.location.is_some() || condition.card_type.is_some() {
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
    /// Cached player reference for "self" target — resolved once at creation.
    self_player: Option<&'a crate::player::Player>,
}

impl<'a> ConditionContext<'a> {
    fn resolve_self_player(
        gs: &'a crate::game_state::GameState,
    ) -> Option<&'a crate::player::Player> {
        gs.activating_card.and_then(|cid| {
            let p1 = &gs.player1;
            let p2 = &gs.player2;
            if p1.stage.stage.contains(&cid)
                || p1.hand.cards.contains(&cid)
                || p1.live_card_zone.cards.contains(&cid)
                || p1.energy_zone.cards.contains(&cid)
            {
                Some(p1)
            } else if p2.stage.stage.contains(&cid)
                || p2.hand.cards.contains(&cid)
                || p2.live_card_zone.cards.contains(&cid)
                || p2.energy_zone.cards.contains(&cid)
            {
                Some(p2)
            } else {
                None
            }
        })
    }

    pub fn new(game_state: &'a crate::game_state::GameState) -> Self {
        let activating_card_id = game_state.activating_card;
        ConditionContext {
            game_state,
            activating_card_id,
            moved_cards: &[],
            selected_card_ids: &[],
            self_player: Self::resolve_self_player(game_state),
        }
    }

    pub fn with_moved_cards(
        game_state: &'a crate::game_state::GameState,
        moved_cards: &'a [i16],
    ) -> Self {
        let activating_card_id = game_state.activating_card;
        ConditionContext {
            game_state,
            activating_card_id,
            moved_cards,
            selected_card_ids: &[],
            self_player: Self::resolve_self_player(game_state),
        }
    }

    pub fn with_moved_and_selected(
        game_state: &'a crate::game_state::GameState,
        moved_cards: &'a [i16],
        selected_card_ids: &'a [i16],
    ) -> Self {
        let activating_card_id = game_state.activating_card;
        ConditionContext {
            game_state,
            activating_card_id,
            moved_cards,
            selected_card_ids,
            self_player: Self::resolve_self_player(game_state),
        }
    }
}

impl<'a> ConditionContext<'a> {
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        // Handle aggregate total with heart_colors — runs before type dispatch
        if condition.aggregate.as_deref() == Some("total")
            && condition.heart_colors.as_ref().is_some_and(|c| !c.is_empty())
            && Zone::from_str(condition.location.as_deref().unwrap_or("")) != Some(Zone::Stage)
        {
            let location = condition.location.as_deref().unwrap_or("");
            let target = condition.target.as_deref().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            if let Some(result) = self.check_aggregate_total(condition, player, location) {
                return result;
            }
        }

        let mut dbg = AbDebug::new();
        let ct = condition.condition_type;
        let result = match ct {
            Some(ConditionType::Compound) => self.evaluate_compound_condition(condition),
            Some(ConditionType::ComparisonCondition) => {
                self.evaluate_comparison_condition(condition)
            }
            Some(ConditionType::LocationCondition) => self.evaluate_location_condition(condition),
            Some(ConditionType::CardCountCondition) => {
                self.evaluate_card_count_condition(condition)
            }
            Some(ConditionType::CardBladeCondition) => {
                self.evaluate_card_blade_condition(condition)
            }
            Some(ConditionType::GroupCondition) => self.evaluate_group_condition(condition),
            Some(ConditionType::PositionCondition) => self.evaluate_position_condition(condition),
            Some(ConditionType::AppearanceCondition) => {
                self.evaluate_appearance_condition(condition)
            }
            Some(ConditionType::TemporalCondition) => self.evaluate_temporal_condition(condition),
            Some(ConditionType::StateCondition) => self.evaluate_state_condition(condition),
            Some(ConditionType::EnergyStateCondition) => {
                self.evaluate_energy_state_condition(condition)
            }
            Some(ConditionType::MovementCondition) => self.evaluate_movement_condition(condition),
            Some(ConditionType::AbilityFilterCondition) => {
                self.evaluate_ability_filter_condition(condition)
            }
            Some(ConditionType::OrCondition) => self.evaluate_or_condition(condition),
            Some(ConditionType::AnyOfCondition) => self.evaluate_any_of_condition(condition),
            Some(ConditionType::ScoreThresholdCondition) => {
                self.evaluate_score_threshold_condition(condition)
            }
            Some(ConditionType::ChoiceCondition) => self.evaluate_choice_condition(condition),
            Some(ConditionType::PositionChangeCondition) => {
                self.evaluate_position_change_condition(condition)
            }
            Some(ConditionType::StateChangeCondition) => {
                self.evaluate_state_change_condition(condition)
            }
            Some(ConditionType::OpponentChoiceCondition) => {
                self.evaluate_opponent_choice_condition(condition)
            }
            Some(ConditionType::OpponentLiveSuccess) => {
                self.evaluate_opponent_live_success_condition(condition)
            }
            Some(ConditionType::ComplexCondition) => self.evaluate_complex_condition(condition),
            Some(ConditionType::NoExcessHeart) => {
                self.evaluate_no_excess_heart_condition(condition)
            }
            Some(ConditionType::OtherwiseCondition) => true, // "otherwise" = catch-all, always true when reached
            Some(ConditionType::NotMoved) | Some(ConditionType::HasMoved) => {
                // These are only meaningful as nested conditions within movement_condition.
                // At top level they are handled by evaluate_movement_condition's nested path.
                false
            }
            Some(ConditionType::ResourceCondition) => self.evaluate_resource_condition(condition),
            Some(ConditionType::ActionSuccessCondition) => {
                // Action success conditions are placed by the parser to gate followup
                // actions on the success of a previous action. When reached here, the
                // previous action already succeeded (otherwise this followup wouldn't
                // be executing). Always passes.
                true
            }
            Some(ConditionType::AllCostComparisonCondition) => {
                self.evaluate_all_cost_comparison_condition(condition)
            }
            Some(ConditionType::Custom) => {
                // Custom conditions are parser-only markers; always true when reached.
                true
            }
            None => false,
        };

        let final_result = if condition.negation.unwrap_or(false)
            && !(ct == Some(ConditionType::CardCountCondition) && condition.card_property.is_some())
        {
            // CardCountCondition with card_property handles negation internally
            // (per-card filter already inverts via negate flag) — do NOT double-negate.
            !result
        } else {
            result
        };
        let thresh = if ct == Some(ConditionType::ComparisonCondition) {
            condition.count.unwrap_or(0)
        } else {
            1
        };
        let dbg_actual = if result {
            condition.count.unwrap_or(1)
        } else {
            0
        };
        dbg.condition(condition, dbg_actual, thresh, final_result);

        // Check ability_filter field on any condition type
        if let Some(ref filter) = condition.ability_filter {
            let filtered =
                self.evaluate_ability_filter_condition_with_card_check(condition, filter);
            if !filtered {
                return false;
            }
        }

        final_result
    }
}

mod card;
mod compound;
mod state;
