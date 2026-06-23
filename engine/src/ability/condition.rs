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
        // Handle aggregate total with heart_colors — runs before type dispatch.
        // Skip early return for TemporalCondition so the phase gate is checked too.
        if condition.condition_type != Some(ConditionType::TemporalCondition)
            && condition.aggregate.as_deref() == Some("total")
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
            Some(ConditionType::BothCondition) => self.evaluate_both_condition(condition),
            Some(ConditionType::Custom) => true,
            Some(ConditionType::NotMoved) | Some(ConditionType::HasMoved) => false,
            // Compound & OrCondition handled above via early return — never reachable here
            Some(ConditionType::Compound) | Some(ConditionType::OrCondition) => unreachable!(),
            None => false,
        };

        let final_result = if condition.negation.unwrap_or(false)
            && !(ct == Some(ConditionType::CardCountCondition) && condition.card_property.is_some())
            && !(ct == Some(ConditionType::LocationCondition)
                && condition.heart_type.as_deref() == Some("all"))
            && !(ct == Some(ConditionType::LocationCondition)
                && condition.location.as_deref() == Some("revealed_cards")
                && self.game_state.revealed_cards.is_empty())
        {
            !result
        } else {
            result
        };
        // Push ONE verdict per condition with actual game state value
        let actual = self.describe_condition_actual(condition);
        push_cond_verdict(condition, &actual, final_result, vec![]);
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

        if let Some(ref filter) = condition.ability_filter {
            let filtered =
                self.evaluate_ability_filter_condition_with_card_check(condition, filter);
            if !filtered {
                return false;
            }
        }

        final_result
    }

    /// Query game state to produce a human-readable "actual" string for this condition.
    /// This runs immediately after evaluation (game state is fresh).
    fn describe_condition_actual(&self, condition: &Condition) -> String {
        let ct = condition.condition_type;
        match ct {
            Some(ConditionType::AppearanceCondition) => self.describe_appearance_actual(condition),
            Some(ConditionType::ComparisonCondition) => {
                let count = self.get_count_for_condition(condition);
                format!("{}", count)
            }
            Some(ConditionType::BothCondition) => {
                let count = self.get_count_for_condition(condition);
                let vals = condition
                    .values
                    .as_ref()
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default();
                format!("count={}, values={}", count, vals)
            }
            Some(ConditionType::CardCountCondition) => {
                let count = self.get_count_for_condition(condition);
                format!("{}", count)
            }
            Some(ConditionType::CardBladeCondition) => {
                if let Some(op) = condition.operator.as_deref() {
                    format!("{} {} {}", "ブレード", op, condition.count.unwrap_or(1))
                } else {
                    String::new()
                }
            }
            Some(ConditionType::GroupCondition) => {
                let player =
                    self.resolve_condition_player(condition.target.as_deref().unwrap_or("self"));
                let loc = condition.location.as_deref().unwrap_or("stage");
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
                            .map(|c| c.name.clone())
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
                    self.resolve_condition_player(condition.target.as_deref().unwrap_or("self"));
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
                                .map(|c| c.name.clone())
                                .unwrap_or_default();
                            format!("{}:{}", pos_names[*i], name)
                        })
                        .collect();
                    desc.join(", ")
                }
            }
            Some(ConditionType::LocationCondition) => {
                let loc = condition.location.as_deref().unwrap_or("");
                if let Some(ref pos) = condition.position {
                    let pos_str = pos.get_position().unwrap_or("?");
                    format!("位置={}", pos_str)
                } else {
                    format!("{}", loc)
                }
            }
            Some(ConditionType::StateCondition) => {
                condition.state.as_deref().unwrap_or("状態").to_string()
            }
            Some(ConditionType::MovementCondition) => {
                let mov = condition.movement.as_deref().unwrap_or("?");
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
                if self.no_excess_heart_flag(condition.target.as_deref().unwrap_or("self")) {
                    "余剰ハートなし".into()
                } else {
                    "余剰ハートあり".into()
                }
            }
            Some(ConditionType::AnyOfCondition) => {
                if let Some(ref any_of) = condition.any_of {
                    format!("条件={:?}", any_of)
                } else {
                    String::new()
                }
            }
            Some(ConditionType::ChoiceCondition) => {
                if let Some(ref opts) = condition.options {
                    format!("選択肢={}個", opts.len())
                } else {
                    "選択肢なし".into()
                }
            }
            Some(ConditionType::EnergyStateCondition) => condition
                .state
                .as_deref()
                .map(|s| format!("エネルギー状態={}", s))
                .unwrap_or_default(),
            Some(ConditionType::StateChangeCondition) => {
                let from = condition.from_state.as_deref().unwrap_or("?");
                let to = condition.to_state.as_deref().unwrap_or("?");
                format!("状態変化: {}→{}", from, to)
            }
            Some(ConditionType::AllCostComparisonCondition) => {
                let op = condition.operator.as_deref().unwrap_or(">");
                format!("全コスト比較{}?", op)
            }
            Some(ConditionType::ScoreThresholdCondition) => {
                let op = condition.operator.as_deref().unwrap_or(">=");
                format!("スコア{} {}?", op, condition.count.unwrap_or(1))
            }
            Some(ConditionType::ResourceCondition) => {
                format!("資源={}", condition.resource_type.as_deref().unwrap_or("?"))
            }
            Some(ConditionType::BothCondition) => {
                let vals = condition
                    .values
                    .as_ref()
                    .map(|v| format!("{:?}", v))
                    .unwrap_or_default();
                format!("両方持つ? values={}", vals)
            }
            _ => String::new(),
        }
    }

    fn describe_appearance_actual(&self, condition: &Condition) -> String {
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let location = condition.location.as_deref().unwrap_or("");

        // Check position constraints first
        let mut position_str = String::new();
        if let Some(ref pos) = condition.position {
            position_str = format!("位置={}", pos.get_position().unwrap_or("?"));
        } else if let Some(ref act_pos) = condition.activation_position {
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
                if let Some(ref chars) = condition.characters {
                    for ch in chars {
                        let norm = crate::card::CardDatabase::normalize_name(ch);
                        let found = stage_names.iter().any(|n| n.contains(&norm));
                        if !found {
                            return format!("{}不在 {}", ch, position_str).trim().to_string();
                        }
                    }
                    // All matched — check cost_reference
                    if let Some(ref ref_char) = condition.cost_reference_character {
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
                        let op = condition.cost_reference_operator.as_deref().unwrap_or(">");
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
                                    .map(|c| c.name.clone())
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
                                .map(|c| c.name.clone())
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
