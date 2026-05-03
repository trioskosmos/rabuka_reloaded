use crate::card::Condition;
use crate::condition_enum::ConditionEnum;
use crate::game_state::Phase;
use super::util;
use super::util::compare_counts;

#[allow(dead_code)]
impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        let cond_enum = ConditionEnum::from_condition(condition);
        let result = match cond_enum {
            ConditionEnum::Compound { .. } => self.evaluate_compound_condition(condition),
            ConditionEnum::Comparison { .. } => self.evaluate_comparison_condition(condition),
            ConditionEnum::Location { .. } => self.evaluate_location_condition(condition),
            ConditionEnum::Position { .. } => self.evaluate_position_condition(condition),
            ConditionEnum::Group { .. } => self.evaluate_group_condition(condition),
            ConditionEnum::CardCount { .. } => self.evaluate_card_count_condition(condition),
            ConditionEnum::Appearance { .. } => self.evaluate_appearance_condition(condition),
            ConditionEnum::Temporal { .. } => self.evaluate_temporal_condition(condition),
            ConditionEnum::State { .. } => self.evaluate_state_condition(condition),
            ConditionEnum::EnergyState { .. } => self.evaluate_energy_state_condition(condition),
            ConditionEnum::Movement { .. } => self.evaluate_movement_condition(condition),
            ConditionEnum::AbilityNegation { .. } => self.evaluate_ability_negation_condition(condition),
            ConditionEnum::Or { .. } => self.evaluate_or_condition(condition),
            ConditionEnum::AnyOf { .. } => self.evaluate_any_of_condition(condition),
            ConditionEnum::ScoreThreshold { .. } => self.evaluate_score_threshold_condition(condition),
            ConditionEnum::Choice => self.evaluate_choice_condition(condition),
            ConditionEnum::PositionChange { .. } => self.evaluate_position_change_condition(condition),
            ConditionEnum::StateChange { .. } => self.evaluate_state_change_condition(condition),
            ConditionEnum::OpponentChoice { .. } => self.evaluate_opponent_choice_condition(condition),
            ConditionEnum::OpponentLiveSuccess => self.evaluate_opponent_live_success_condition(condition),
            ConditionEnum::Complex => self.evaluate_complex_condition(condition),
        };

        if condition.negation.unwrap_or(false) {
            !result
        } else {
            result
        }
    }

    fn evaluate_compound_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            match condition.operator.as_deref() {
                Some("and") => conditions.iter().all(|c| self.evaluate_condition(c)),
                Some("or") => conditions.iter().any(|c| self.evaluate_condition(c)),
                _ => true,
            }
        } else {
            true
        }
    }

    fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        let count = self.get_count_for_condition(condition);

        if let Some(ref values) = condition.values {
            return values.contains(&(count as u32));
        }

        let default_count = if condition.location.is_some() || condition.card_type.is_some() { 1 } else { 0 };
        let target_count = if let Some(ref comparison_target) = condition.comparison_target {
            if comparison_target == "opponent" {
                self.get_count_for_target(condition, "opponent")
            } else {
                condition.count.unwrap_or(default_count)
            }
        } else {
            condition.count.unwrap_or(default_count)
        };

        compare_counts(condition.operator.as_deref(), count, target_count)
    }

    fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let card_type_filter = condition.card_type.as_deref();
        let comparison_type = condition.comparison_type.as_deref();
        let operator = condition.operator.as_deref();
        // When count is not specified but filters are set, default to 1
        // (checking "at least one matching card exists") instead of 0
        // which would make >= always true.
        let count_threshold = condition.count.unwrap_or(if condition.cost_limit.is_some() || condition.card_type.is_some() || condition.group_names.is_some() || condition.characters.is_some() || condition.distinct.unwrap_or(false) || condition.appearance.unwrap_or(false) { 1 } else { 0 });
        let distinct = condition.distinct.unwrap_or(false);
        let all_areas = condition.all_areas.unwrap_or(false);
        let no_excess_heart = condition.no_excess_heart.unwrap_or(false);
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);
        let cost_limit = condition.cost_limit;
        let group_names = condition.group_names.as_ref();
        let _characters = condition.characters.as_ref();

        if baton_touch_trigger {
            if self.game_state.baton_touch_count == 0 {
                return false;
            }
        }

        let card_db = &self.game_state.card_database;

        let check_distinct_names = |card_ids: &[i16]| -> bool {
            let mut names = std::collections::HashSet::new();
            for &card_id in card_ids {
                if card_id == -1 { continue; }
                if card_db.get_card(card_id).is_some() {
                    let card_names = card_db.get_card_names(card_id);
                    for name in card_names {
                        if !names.insert(name) {
                            return false;
                        }
                    }
                }
            }
            true
        };

        let location_value = match target {
            "either" => {
                if comparison_type == Some("score") || comparison_type == Some("cost") {
                    let v1 = self.get_count_for_target(condition, "self");
                    let v2 = self.get_count_for_target(condition, "opponent");
                    v1.max(v2)
                } else {
                    let p1_cards: &[i16] = match location {
                        "stage" => &self.game_state.player1.stage.stage, "hand" => &self.game_state.player1.hand.cards,
                        "deck" => &self.game_state.player1.main_deck.cards, "discard" | "waitroom" => &self.game_state.player1.waitroom.cards,
                        "energy_zone" => &self.game_state.player1.energy_zone.cards, "live_card_zone" => &self.game_state.player1.live_card_zone.cards,
                        "success_live_zone" => &self.game_state.player1.success_live_card_zone.cards, _ => &[],
                    };
                    let p2_cards: &[i16] = match location {
                        "stage" => &self.game_state.player2.stage.stage, "hand" => &self.game_state.player2.hand.cards,
                        "deck" => &self.game_state.player2.main_deck.cards, "discard" | "waitroom" => &self.game_state.player2.waitroom.cards,
                        "energy_zone" => &self.game_state.player2.energy_zone.cards, "live_card_zone" => &self.game_state.player2.live_card_zone.cards,
                        "success_live_zone" => &self.game_state.player2.success_live_card_zone.cards, _ => &[],
                    };
                    let c1 = util::count_matching(p1_cards, card_db, card_type_filter, group_names.and_then(|g| g.first().map(|s| s.as_str())), cost_limit, operator);
                    let c2 = util::count_matching(p2_cards, card_db, card_type_filter, group_names.and_then(|g| g.first().map(|s| s.as_str())), cost_limit, operator);
                    if all_areas {
                        let p1_stage: &[i16] = &self.game_state.player1.stage.stage;
                        let p2_stage: &[i16] = &self.game_state.player2.stage.stage;
                        if p1_stage.iter().filter(|&&c| c != -1).count() != 3
                            && p2_stage.iter().filter(|&&c| c != -1).count() != 3 {
                            return false;
                        }
                    }
                    c1.max(c2)
                }
            }
            _ => {
                let player = self.game_state.resolve_target_player(target);
                let cards: &[i16] = match location {
                    "stage" => &player.stage.stage, "hand" => &player.hand.cards,
                    "deck" => &player.main_deck.cards, "discard" | "waitroom" => &player.waitroom.cards,
                    "energy_zone" => &player.energy_zone.cards, "live_card_zone" => &player.live_card_zone.cards,
                    "success_live_zone" => &player.success_live_card_zone.cards, _ => &[],
                };
                if comparison_type == Some("score") || comparison_type == Some("cost") || comparison_type == Some("energy") {
                    self.get_count_for_target(condition, target)
                } else {
                    let c = util::count_matching(cards, card_db, card_type_filter, group_names.and_then(|g| g.first().map(|s| s.as_str())), cost_limit, operator);
                    if all_areas {
                        let stage_slice: &[i16] = &player.stage.stage;
                        if stage_slice.iter().filter(|&&c| c != -1).count() != 3 {
                            return false;
                        }
                    }
                    c
                }
            }
        };

        if distinct {
            let player = self.game_state.resolve_target_player(if target == "either" { "self" } else { target });
            let card_ids: Vec<i16> = match location {
                "stage" => player.stage.stage.to_vec(),
                "hand" => player.hand.cards.to_vec(),
                "deck" => player.main_deck.cards.to_vec(),
                "discard" | "waitroom" => player.waitroom.cards.to_vec(),
                "energy_zone" => player.energy_zone.cards.to_vec(),
                "live_card_zone" => player.live_card_zone.cards.to_vec(),
                "success_live_zone" => player.success_live_card_zone.cards.to_vec(),
                _ => vec![],
            };
            if !check_distinct_names(&card_ids) {
                return false;
            }
        }

        if no_excess_heart {
            let opponent = self.game_state.resolve_target_player(if target == "self" { "opponent" } else { "self" });
            let total_hearts: u32 = opponent.stage.stage.iter()
                .filter(|&&card_id| card_id != -1)
                .map(|&card_id| card_db.get_card(card_id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            let needed_hearts: u32 = opponent.live_card_zone.cards.iter()
                .map(|&card_id| card_db.get_card(card_id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            if total_hearts > needed_hearts {
                return false;
            }
        }

        compare_counts(operator, location_value, count_threshold)
    }

    fn evaluate_position_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        let position = condition.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        let player = self.game_state.resolve_target_player(target);
        match position {
            "center" => player.stage.stage[1] != -1,
            "left_side" => player.stage.stage[0] != -1,
            "right_side" => player.stage.stage[2] != -1,
            "any" => player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1,
            _ => true,
        }
    }

    fn evaluate_group_condition(&self, condition: &Condition) -> bool {
        let count = self.get_group_card_count(condition);
        // Default to 1 when count is not set (checking "at least one matching")
        let target_count = condition.count.unwrap_or(1);
        compare_counts(condition.operator.as_deref(), count, target_count)
    }

    fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
        let card_type = condition.card_type.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        // Default to 1 when count is not set (checking "at least one of this type")
        let count = condition.count.unwrap_or(1);
        let player = self.game_state.resolve_target_player(target);
        let actual_count = match card_type {
            "live_card" => player.live_card_zone.len(),
            "member_card" => player.stage.total_blades(&self.game_state.card_database, &self.game_state.blade_modifiers) as usize,
            "energy_card" => player.energy_zone.cards.len(),
            _ => 0,
        };
        compare_counts(condition.operator.as_deref(), actual_count as u32, count)
    }

    fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
        let appearance = condition.appearance.unwrap_or(false);
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);
        let player = self.game_state.resolve_target_player(target);

        if baton_touch_trigger {
            if let Some(ref _activating_card) = self.game_state.activating_card {
                return self.game_state.baton_touch_count > 0;
            }
            return false;
        }

        if appearance {
            match location {
                "stage" => player.stage.stage.iter().any(|&card_id| card_id != -1),
                "hand" => !player.hand.cards.is_empty(),
                "discard" => !player.waitroom.cards.is_empty(),
                _ => true,
            }
        } else {
            match location {
                "stage" => player.stage.stage[0] == -1 && player.stage.stage[1] == -1 && player.stage.stage[2] == -1,
                "hand" => player.hand.cards.is_empty(),
                "discard" => player.waitroom.cards.is_empty(),
                _ => true,
            }
        }
    }

    fn evaluate_temporal_condition(&self, condition: &Condition) -> bool {
        let temporal = condition.temporal.as_deref().unwrap_or("");
        let phase = condition.phase.as_deref();

        match temporal {
            "this_turn" => {
                if let Some(created_turn) = condition.temporal_scope.as_ref().and_then(|s| s.parse::<u32>().ok()) {
                    created_turn == self.game_state.turn_number
                } else {
                    if let Some(nested_condition) = &condition.condition {
                        match nested_condition.condition_type.as_deref() {
                            Some("not_moved") => {
                                if let Some(activating_card_id) = self.activating_card_id {
                                    !self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else { true }
                            }
                            Some("has_moved") => {
                                if let Some(activating_card_id) = self.activating_card_id {
                                    self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else { false }
                            }
                            _ => self.evaluate_condition(nested_condition),
                        }
                    } else { true }
                }
            }
            "live_end" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination),
            "this_live" => {
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP1Turn) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP2Turn) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "before_live" => {
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP1Turn) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP2Turn) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "first_turn" => self.game_state.is_first_turn,
            _ => {
                if let Some(phase_str) = phase {
                    match phase_str {
                        "active" => matches!(self.game_state.current_phase, crate::game_state::Phase::Active),
                        "live_card_set" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSetP1Turn | crate::game_state::Phase::LiveCardSetP2Turn),
                        "live_performance" => matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) ||
                                               matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance),
                        "live_victory" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination),
                        _ => true,
                    }
                } else { true }
            }
        }
    }

    fn evaluate_state_condition(&self, condition: &Condition) -> bool {
        let state = condition.state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        match state {
            "active" | "wait" => player.stage.stage.iter().any(|&card_id| card_id != -1),
            _ => true,
        }
    }

    fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        let energy_state = condition.energy_state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        match energy_state {
            "active" => player.energy_zone.active_count() > 0,
            _ => true,
        }
    }

    fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
        let movement = condition.movement.as_deref().unwrap_or("");
        let movement_state = condition.movement_state.as_deref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);

        match movement {
            "moved" => {
                if let Some(state) = movement_state {
                    match state {
                        "to_stage" => player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1,
                        "from_stage" => !player.waitroom.cards.is_empty(),
                        "to_discard" => !player.waitroom.cards.is_empty(),
                        _ => true,
                    }
                } else { true }
            }
            "notmoved" => true,
            "baton_touch" => condition.baton_touch_trigger.unwrap_or(false),
            _ => {
                match location {
                    "stage" => player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1,
                    "hand" => !player.hand.cards.is_empty(),
                    "discard" => !player.waitroom.cards.is_empty(),
                    _ => true,
                }
            }
        }
    }

    fn evaluate_ability_negation_condition(&self, condition: &Condition) -> bool {
        let negation = condition.negation.unwrap_or(false);
        if negation {
            self.game_state.prohibition_effects.is_empty()
        } else { true }
    }

    fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            conditions.iter().any(|c| self.evaluate_condition(c))
        } else { true }
    }

    fn evaluate_any_of_condition(&self, condition: &Condition) -> bool {
        if let Some(ref any_of) = condition.any_of {
            any_of.iter().any(|condition_type| {
                let master = &*self.game_state.resolve_target_player("self");
                match condition_type.as_str() {
                    "has_member" => !master.stage.stage.iter().all(|&id| id == crate::constants::EMPTY_SLOT),
                    "has_energy" => !master.energy_zone.cards.is_empty(),
                    "has_hand" => !master.hand.cards.is_empty(),
                    "has_blade_heart" => master.stage.stage.iter().any(|&id| {
                        id != crate::constants::EMPTY_SLOT && self.game_state.card_database.get_card(id).map(|c| c.has_blade_heart()).unwrap_or(false)
                    }),
                    "has_live_card" => !master.live_card_zone.cards.is_empty(),
                    "is_active_phase" => matches!(self.game_state.current_phase, crate::game_state::Phase::Active),
                    "is_main_phase" => matches!(self.game_state.current_phase, crate::game_state::Phase::Main),
                    _ => { eprintln!("Unknown any_of condition type: {}", condition_type); false }
                }
            })
        } else { true }
    }

    fn evaluate_score_threshold_condition(&self, condition: &Condition) -> bool {
        // Default to 1 when count is not set (checking "score exists")
        let count = condition.count.unwrap_or(1);
        let operator = condition.operator.as_deref();
        let target = condition.target.as_deref().unwrap_or("self");
        let cheer_count = if target == "self" { self.game_state.player1_cheer_blade_heart_count } else if target == "opponent" { self.game_state.player2_cheer_blade_heart_count } else { self.game_state.player1_cheer_blade_heart_count };
        compare_counts(operator, cheer_count, count)
    }

    fn evaluate_choice_condition(&self, condition: &Condition) -> bool {
        if let Some(ref options) = condition.options {
            !options.is_empty()
        } else {
            true
        }
    }

    fn evaluate_position_change_condition(&self, condition: &Condition) -> bool {
        let optional = condition.options.as_ref().map(|_| true).unwrap_or(false);
        if optional {
            if self.game_state.position_change_occurred_this_turn {
                return true;
            }
            return false;
        }
        self.game_state.position_change_occurred_this_turn
    }

    fn evaluate_state_change_condition(&self, condition: &Condition) -> bool {
        let _text = &condition.text;
        let _during_main_phase = condition.text.contains("main_phase");
        if _during_main_phase && self.game_state.current_phase != Phase::Main {
            return false;
        }
        true
    }

    fn evaluate_opponent_choice_condition(&self, condition: &Condition) -> bool {
        let _target = condition.target.as_deref().unwrap_or("opponent");
        let negation = condition.negation.unwrap_or(false);
        let opponent_declined = self.game_state.opponent_choice_declined;
        if negation { opponent_declined } else { !opponent_declined }
    }

    fn evaluate_opponent_live_success_condition(&self, _condition: &Condition) -> bool {
        self.game_state.opponent_live_success_this_turn
    }

    fn evaluate_complex_condition(&self, condition: &Condition) -> bool {
        if let Some(ref cause) = condition.cause {
            self.evaluate_condition(cause)
        } else {
            true
        }
    }

    fn zone_len(&self, player: &crate::player::Player, location: &str) -> u32 {
        match location {
            "stage" => player.stage.total_blades(&self.game_state.card_database, &self.game_state.blade_modifiers),
            "hand" => player.hand.len() as u32,
            "deck" => player.main_deck.len() as u32,
            "discard" => player.waitroom.len() as u32,
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => player.live_card_zone.len() as u32,
            "success_live_zone" => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    fn get_count_for_condition(&self, condition: &Condition) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        self.zone_len(player, location)
    }

    fn get_count_for_target(&self, condition: &Condition, target: &str) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let comparison_type = condition.comparison_type.as_deref();
        let player = self.game_state.resolve_target_player(target);

        if let Some(comp_type) = comparison_type {
            match comp_type {
                "score" => {
                    let mut total_score = 0;
                    for card_id in &player.success_live_card_zone.cards {
                        if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                            total_score += card.score.unwrap_or(0);
                        }
                    }
                    total_score
                }
                "cost" => {
                    let mut total_cost = 0;
                    for card_id in &player.stage.stage {
                        if *card_id != -1 {
                            if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                                total_cost += card.cost.unwrap_or(0);
                            }
                        }
                    }
                    total_cost
                }
                "energy" => player.energy_zone.cards.len() as u32,
                _ => self.zone_len(player, location),
            }
        } else {
            self.zone_len(player, location)
        }
    }

    fn get_group_card_count(&self, condition: &Condition) -> u32 {
        let group_filter = condition.group_names.as_ref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        let mut count = 0;
        let card_db = self.game_state.card_database.clone();

        let matches_group = |card_id: i16, groups: Option<&Vec<String>>| -> bool {
            match groups {
                Some(group_names) => card_db.get_card(card_id).map(|c| group_names.iter().any(|g| c.group == *g)).unwrap_or(false),
                None => true,
            }
        };

        match location {
            "stage" => {
                for i in 0..3 {
                    if player.stage.stage[i] != -1 && matches_group(player.stage.stage[i], group_filter) {
                        count += 1;
                    }
                }
            }
            "hand" => {
                for card in &player.hand.cards {
                    if matches_group(*card, group_filter) { count += 1; }
                }
            }
            "discard" | "waitroom" => {
                for card in &player.waitroom.cards {
                    if matches_group(*card, group_filter) { count += 1; }
                }
            }
            _ => {}
        }
        count
    }
}
