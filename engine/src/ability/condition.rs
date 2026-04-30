use crate::card::Condition;
use crate::game_state::Phase;

#[allow(dead_code)]
impl<'a> super::resolver::AbilityResolver<'a> {
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        let result = match condition.condition_type.as_deref() {
            Some("compound") => self.evaluate_compound_condition(condition),
            Some("comparison_condition") => self.evaluate_comparison_condition(condition),
            Some("location_condition") => self.evaluate_location_condition(condition),
            Some("position_condition") => self.evaluate_position_condition(condition),
            Some("group_condition") => self.evaluate_group_condition(condition),
            Some("card_count_condition") => self.evaluate_card_count_condition(condition),
            Some("appearance_condition") => self.evaluate_appearance_condition(condition),
            Some("temporal_condition") => self.evaluate_temporal_condition(condition),
            Some("state_condition") => self.evaluate_state_condition(condition),
            Some("energy_state_condition") => self.evaluate_energy_state_condition(condition),
            Some("movement_condition") => self.evaluate_movement_condition(condition),
            Some("ability_negation_condition") => self.evaluate_ability_negation_condition(condition),
            Some("or_condition") => self.evaluate_or_condition(condition),
            Some("any_of_condition") => self.evaluate_any_of_condition(condition),
            Some("score_threshold_condition") => self.evaluate_score_threshold_condition(condition),
            Some("choice_condition") => self.evaluate_choice_condition(condition),
            Some("position_change_condition") => self.evaluate_position_change_condition(condition),
            Some("state_change_condition") => self.evaluate_state_change_condition(condition),
            Some("opponent_choice_condition") => self.evaluate_opponent_choice_condition(condition),
            _ => {
                eprintln!("Unknown condition type: {:?}", condition.condition_type);
                true
            }
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

        let target_count = if let Some(ref comparison_target) = condition.comparison_target {
            if comparison_target == "opponent" {
                self.get_count_for_target(condition, "opponent")
            } else {
                condition.count.unwrap_or(0)
            }
        } else {
            condition.count.unwrap_or(0)
        };

        match condition.operator.as_deref() {
            Some(">=") => count >= target_count,
            Some(">") => count > target_count,
            Some("<=") => count <= target_count,
            Some("<") => count < target_count,
            Some("==") | Some("=") => count == target_count,
            Some("!=") => count != target_count,
            _ => true,
        }
    }

    fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let card_type_filter = condition.card_type.as_deref();
        let aggregate = condition.aggregate.as_deref();
        let comparison_type = condition.comparison_type.as_deref();
        let operator = condition.operator.as_deref();
        let count_threshold = condition.count.unwrap_or(0);
        let distinct = condition.distinct.unwrap_or(false);
        let all_areas = condition.all_areas.unwrap_or(false);
        let no_excess_heart = condition.no_excess_heart.unwrap_or(false);
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);

        if baton_touch_trigger {
            if self.game_state.baton_touch_count == 0 {
                return false;
            }
        }

        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        let card_db = &self.game_state.card_database;

        let matches_card_type = |card_id: i16| -> bool {
            if let Some(card) = card_db.get_card(card_id) {
                match card_type_filter.unwrap_or("") {
                    "member_card" | "メンバー" => card.is_member(),
                    "live_card" | "ライブ" => card.is_live(),
                    _ => true,
                }
            } else {
                false
            }
        };

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

        let location_value = match location {
            "stage" => {
                if comparison_type == Some("score") {
                    let mut total_score = 0u32;
                    if player.stage.stage[1] != -1 {
                        if let Some(card) = card_db.get_card(player.stage.stage[1]) {
                            total_score += card.get_score() + self.game_state.get_score_modifier(player.stage.stage[1]) as u32;
                        }
                    }
                    if player.stage.stage[0] != -1 {
                        if let Some(card) = card_db.get_card(player.stage.stage[0]) {
                            total_score += card.get_score() + self.game_state.get_score_modifier(player.stage.stage[0]) as u32;
                        }
                    }
                    if player.stage.stage[2] != -1 {
                        if let Some(card) = card_db.get_card(player.stage.stage[2]) {
                            total_score += card.get_score() + self.game_state.get_score_modifier(player.stage.stage[2]) as u32;
                        }
                    }
                    total_score
                } else {
                    let count = player.stage.stage.iter().filter(|&&card_id| card_id != -1).count();
                    if all_areas && count != 3 {
                        return false;
                    }
                    count as u32
                }
            }
            "hand" => {
                if comparison_type == Some("score") {
                    player.hand.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else if card_type_filter.is_some() {
                    player.hand.cards.iter().filter(|&id| matches_card_type(*id)).count() as u32
                } else {
                    player.hand.cards.len() as u32
                }
            }
            "deck" => player.main_deck.cards.len() as u32,
            "discard" => {
                if comparison_type == Some("score") {
                    player.waitroom.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else if card_type_filter.is_some() {
                    player.waitroom.cards.iter().filter(|&id| matches_card_type(*id)).count() as u32
                } else {
                    player.waitroom.cards.len() as u32
                }
            }
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => {
                if comparison_type == Some("score") {
                    player.live_card_zone.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else {
                    player.live_card_zone.cards.len() as u32
                }
            }
            "success_live_zone" => {
                if comparison_type == Some("score") {
                    player.success_live_card_zone.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else {
                    player.success_live_card_zone.cards.len() as u32
                }
            }
            "revealed_cards" => {
                let revealed_count = self.game_state.revealed_cards.len() as u32;
                if let Some(property) = condition.card_property.as_deref() {
                    match property {
                        "has_blade_heart" => {
                            self.game_state.revealed_cards.iter()
                                .filter(|&&card_id| {
                                    card_db.get_card(card_id).map(|c| c.has_blade_heart()).unwrap_or(false)
                                })
                                .count() as u32
                        }
                        _ => revealed_count
                    }
                } else {
                    revealed_count
                }
            }
            _ => 0,
        };

        if distinct {
            let card_ids: Vec<i16> = match location {
                "stage" => player.stage.stage.to_vec(),
                "hand" => player.hand.cards.to_vec(),
                "discard" => player.waitroom.cards.to_vec(),
                "energy_zone" => player.energy_zone.cards.to_vec(),
                "live_card_zone" => player.live_card_zone.cards.to_vec(),
                "success_live_zone" => player.success_live_card_zone.cards.to_vec(),
                _ => Vec::new(),
            };
            if !check_distinct_names(&card_ids) {
                return false;
            }
        }

        if no_excess_heart {
            let opponent = if target == "self" { &self.game_state.player2 } else { &self.game_state.player1 };
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

        let final_value = match aggregate {
            Some("total") => location_value,
            None => location_value,
            _ => location_value,
        };

        match operator {
            Some(">=") => final_value >= count_threshold,
            Some(">") => final_value > count_threshold,
            Some("<=") => final_value <= count_threshold,
            Some("<") => final_value < count_threshold,
            Some("==") | Some("=") => final_value == count_threshold,
            Some("!=") => final_value != count_threshold,
            None => final_value > 0,
            _ => final_value > 0,
        }
    }

    fn evaluate_position_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        let position = condition.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
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
        let target_count = condition.count.unwrap_or(0);
        match condition.operator.as_deref() {
            Some(">=") => count >= target_count,
            Some(">") => count > target_count,
            Some("<=") => count <= target_count,
            Some("<") => count < target_count,
            Some("==") | Some("=") => count == target_count,
            Some("!=") => count != target_count,
            _ => true,
        }
    }

    fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
        let card_type = condition.card_type.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let count = condition.count.unwrap_or(0);
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
        let actual_count = match card_type {
            "live_card" => player.live_card_zone.len(),
            "member_card" => player.stage.total_blades(&self.game_state.card_database) as usize,
            "energy_card" => player.energy_zone.cards.len(),
            _ => 0,
        };
        match condition.operator.as_deref() {
            Some(">=") => actual_count as u32 >= count,
            Some(">") => actual_count as u32 > count,
            Some("<=") => actual_count as u32 <= count,
            Some("<") => (actual_count as u32) < count,
            Some("==") | Some("=") => actual_count as u32 == count,
            Some("!=") => actual_count as u32 != count,
            _ => true,
        }
    }

    fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
        let appearance = condition.appearance.unwrap_or(false);
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

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
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSet) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "before_live" => {
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSet) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "first_turn" => self.game_state.is_first_turn,
            _ => {
                if let Some(phase_str) = phase {
                    match phase_str {
                        "active" => matches!(self.game_state.current_phase, crate::game_state::Phase::Active),
                        "live_card_set" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSet),
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
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
        match state {
            "active" | "wait" => player.stage.stage.iter().any(|&card_id| card_id != -1),
            _ => true,
        }
    }

    fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        let energy_state = condition.energy_state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
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
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

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
                match condition_type.as_str() {
                    "has_member" => !self.game_state.player1.stage.stage.iter().all(|&id| id == crate::constants::EMPTY_SLOT),
                    "has_energy" => !self.game_state.player1.energy_zone.cards.is_empty(),
                    "has_hand" => !self.game_state.player1.hand.cards.is_empty(),
                    "has_blade_heart" => self.game_state.player1.stage.stage.iter().any(|&id| {
                        id != crate::constants::EMPTY_SLOT && self.game_state.card_database.get_card(id).map(|c| c.has_blade_heart()).unwrap_or(false)
                    }),
                    "has_live_card" => !self.game_state.player1.live_card_zone.cards.is_empty(),
                    "is_active_phase" => matches!(self.game_state.current_phase, crate::game_state::Phase::Active),
                    "is_main_phase" => matches!(self.game_state.current_phase, crate::game_state::Phase::Main),
                    _ => { eprintln!("Unknown any_of condition type: {}", condition_type); false }
                }
            })
        } else { true }
    }

    fn evaluate_score_threshold_condition(&self, condition: &Condition) -> bool {
        let count = condition.count.unwrap_or(0);
        let operator = condition.operator.as_deref();
        let target = condition.target.as_deref().unwrap_or("self");
        let cheer_count = match target {
            "self" => self.game_state.player1_cheer_blade_heart_count,
            "opponent" => self.game_state.player2_cheer_blade_heart_count,
            _ => self.game_state.player1_cheer_blade_heart_count,
        };
        match operator {
            Some(">=") => cheer_count >= count,
            Some(">") => cheer_count > count,
            Some("<=") => cheer_count <= count,
            Some("<") => cheer_count < count,
            Some("==") | Some("=") => cheer_count == count,
            Some("!=") => cheer_count != count,
            _ => true,
        }
    }

    fn evaluate_choice_condition(&self, _condition: &Condition) -> bool {
        true
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

    fn get_count_for_condition(&self, condition: &Condition) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
        match location {
            "stage" => player.stage.total_blades(&self.game_state.card_database),
            "hand" => player.hand.len() as u32,
            "deck" => player.main_deck.len() as u32,
            "discard" => player.waitroom.len() as u32,
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => player.live_card_zone.len() as u32,
            "success_live_zone" => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    fn get_count_for_target(&self, condition: &Condition, target: &str) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let comparison_type = condition.comparison_type.as_deref();
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

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
                _ => {
                    match location {
                        "stage" => player.stage.total_blades(&self.game_state.card_database),
                        "hand" => player.hand.len() as u32,
                        "deck" => player.main_deck.len() as u32,
                        "discard" => player.waitroom.len() as u32,
                        "energy_zone" => player.energy_zone.cards.len() as u32,
                        "live_card_zone" => player.live_card_zone.len() as u32,
                        "success_live_zone" => player.success_live_card_zone.len() as u32,
                        _ => 0,
                    }
                }
            }
        } else {
            match location {
                "stage" => player.stage.total_blades(&self.game_state.card_database),
                "hand" => player.hand.len() as u32,
                "deck" => player.main_deck.len() as u32,
                "discard" => player.waitroom.len() as u32,
                "energy_zone" => player.energy_zone.cards.len() as u32,
                "live_card_zone" => player.live_card_zone.len() as u32,
                "success_live_zone" => player.success_live_card_zone.len() as u32,
                _ => 0,
            }
        }
    }

    fn get_group_card_count(&self, condition: &Condition) -> u32 {
        let group_filter = condition.group_names.as_ref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
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
