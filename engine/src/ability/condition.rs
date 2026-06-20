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

/// Push a condition verdict to the structured log buffer.
/// `actual_label` overrides the auto-generated actual string; use "" to auto-generate.
pub fn push_cond_verdict(
    condition: &Condition,
    extra_actual: &str,
    passed: bool,
    children: Vec<crate::ability::log::AbilityLogItem>,
) {
    use crate::ability::log::{push_verdict, AbilityLogItem};
    let ct = condition.condition_type;
    let condition_type = ct.map(|t| t.to_str().to_string()).unwrap_or_default();
    let op = condition.operator.as_deref().unwrap_or(">=");
    let threshold = condition.count.map(|c| c.to_string()).unwrap_or_default();
    let resource = condition.resource_type.as_deref().unwrap_or("");
    let location = condition.location.as_deref().unwrap_or("");

    let expectation = match ct {
        Some(ConditionType::AppearanceCondition) => {
            if let Some(ref chars) = condition.characters {
                if !chars.is_empty() {
                    if condition.cost_reference_character.is_some() {
                        format!(
                            "{} {} {}",
                            chars[0],
                            condition.cost_reference_operator.as_deref().unwrap_or(">"),
                            condition.cost_reference_character.as_deref().unwrap_or("")
                        )
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
        Some(ConditionType::ComparisonCondition) => {
            if !resource.is_empty() {
                format!("{}{} {}{}", op, threshold, resource, location)
            } else if !location.is_empty() {
                format!("{}{} {}", op, threshold, location)
            } else {
                format!("{}{}", op, threshold)
            }
        }
        Some(ConditionType::CardCountCondition) => {
            let ct_field = condition.card_type.as_deref().unwrap_or("");
            if !ct_field.is_empty() {
                format!("{}{} {} {}", op, threshold, ct_field, location)
            } else if !location.is_empty() {
                format!("{}{} {}", op, threshold, location)
            } else {
                format!("{}{}", op, threshold)
            }
        }
        Some(ConditionType::LocationCondition) => {
            format!("位置={}", location)
        }
        Some(ConditionType::GroupCondition) => {
            if let Some(ref gns) = condition.group_names {
                format!("所属={}", gns.join(","))
            } else {
                "所属条件".into()
            }
        }
        Some(ConditionType::PositionCondition) => {
            if let Some(ref pos) = condition.position {
                format!("位置={}", pos.get_position().unwrap_or("?"))
            } else {
                "位置条件".into()
            }
        }
        Some(ConditionType::CardBladeCondition) => {
            format!("ブレード{}{}", op, threshold)
        }
        Some(ConditionType::ScoreThresholdCondition) => {
            format!("スコア{}{}", op, threshold)
        }
        Some(ConditionType::ResourceCondition) => {
            format!("資源{}{}", op, threshold)
        }
        Some(ConditionType::StateCondition) => {
            condition.state.as_deref().unwrap_or("状態").to_string()
        }
        Some(ConditionType::MovementCondition) => {
            format!("移動={}", condition.movement.as_deref().unwrap_or("?"))
        }
        Some(ConditionType::TemporalCondition) => condition
            .temporal
            .as_deref()
            .unwrap_or("タイミング")
            .to_string(),
        Some(ConditionType::EnergyStateCondition) => condition
            .energy_state
            .as_deref()
            .unwrap_or("エネルギー状態")
            .to_string(),
        Some(ConditionType::AbilityFilterCondition) => condition
            .ability_filter
            .as_deref()
            .unwrap_or("フィルター")
            .to_string(),
        Some(ConditionType::NoExcessHeart) => "余剰ハートなし".into(),
        Some(ConditionType::AllCostComparisonCondition) => {
            format!("全コスト合計{}{}", op, threshold)
        }
        _ => String::new(),
    };

    let actual = if !extra_actual.is_empty() {
        extra_actual.to_string()
    } else if passed {
        "条件満たす".into()
    } else {
        "条件満たさない".into()
    };

    push_verdict(AbilityLogItem::Condition {
        text: condition.text.clone(),
        condition_type,
        expectation,
        actual,
        passed,
        children,
    });
}

impl<'a> ConditionContext<'a> {
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        // Handle aggregate total with heart_colors — runs before type dispatch
        if condition.aggregate.as_deref() == Some("total")
            && condition
                .heart_colors
                .as_ref()
                .is_some_and(|c| !c.is_empty())
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
        // Snapshot buffer before compound/or so children can be collected
        let _before = crate::ability::log::buffer_len();
        // Handle compound/or first — they push their own verdicts with children
        match ct {
            Some(ConditionType::Compound) => {
                let r = self.evaluate_compound_condition(condition);
                return r;
            }
            Some(ConditionType::OrCondition) => {
                let r = self.evaluate_or_condition(condition);
                return r;
            }
            _ => {}
        }
        // For all other types: run evaluator, then push generic verdict
        let result: bool = match ct {
            Some(ConditionType::AppearanceCondition) => {
                self.evaluate_appearance_condition(condition)
            }
            Some(ConditionType::ComparisonCondition) => {
                self.evaluate_comparison_condition(condition)
            }
            Some(ConditionType::CardCountCondition) => {
                self.evaluate_card_count_condition(condition)
            }
            Some(ConditionType::LocationCondition) => self.evaluate_location_condition(condition),
            Some(ConditionType::CardBladeCondition) => {
                self.evaluate_card_blade_condition(condition)
            }
            Some(ConditionType::GroupCondition) => self.evaluate_group_condition(condition),
            Some(ConditionType::PositionCondition) => self.evaluate_position_condition(condition),
            Some(ConditionType::TemporalCondition) => self.evaluate_temporal_condition(condition),
            Some(ConditionType::MovementCondition) => self.evaluate_movement_condition(condition),
            Some(ConditionType::StateCondition) => self.evaluate_state_condition(condition),
            Some(ConditionType::EnergyStateCondition) => {
                self.evaluate_energy_state_condition(condition)
            }
            Some(ConditionType::AbilityFilterCondition) => {
                self.evaluate_ability_filter_condition(condition)
            }
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
            Some(ConditionType::ResourceCondition) => self.evaluate_resource_condition(condition),
            Some(ConditionType::AllCostComparisonCondition) => {
                self.evaluate_all_cost_comparison_condition(condition)
            }
            Some(ConditionType::OtherwiseCondition) => true,
            Some(ConditionType::ActionSuccessCondition) => true,
            Some(ConditionType::Custom) => true,
            Some(ConditionType::NotMoved) | Some(ConditionType::HasMoved) => false,
            // Compound & OrCondition handled above via early return — never reachable here
            Some(ConditionType::Compound) | Some(ConditionType::OrCondition) => unreachable!(),
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
        // Push generic verdict for this condition (enriched verdicts from evaluators
        // go to the buffer first and are deduplicated by the resolver).
        push_cond_verdict(condition, "", final_result, vec![]);
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
