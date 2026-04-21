use crate::card::{Ability, AbilityCost, AbilityEffect, Condition};
use crate::game_state::GameState;

pub struct AbilityResolver<'a> {
    game_state: &'a mut GameState,
}

impl<'a> AbilityResolver<'a> {
    pub fn new(game_state: &'a mut GameState) -> Self {
        AbilityResolver { game_state }
    }

    /// Evaluate a condition against the current game state
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition.condition_type.as_deref() {
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
            _ => {
                // Default: unknown condition type, return true (fail-open)
                eprintln!("Unknown condition type: {:?}", condition.condition_type);
                true
            }
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

    fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        // Check if cards exist in specified location
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let card_type_filter = condition.card_type.as_deref();
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        let matches_card_type = |card: &crate::card::Card| -> bool {
            match card_type_filter {
                Some("live_card") => card.is_live(),
                Some("member_card") => card.is_member(),
                Some("energy_card") => card.is_energy(),
                None => true,
                _ => true,
            }
        };

        match location {
            "stage" => {
                if card_type_filter.is_some() {
                    // Check for specific card type on stage
                    player.stage.center.as_ref().map_or(false, |c| matches_card_type(&c.card)) ||
                    player.stage.left_side.as_ref().map_or(false, |c| matches_card_type(&c.card)) ||
                    player.stage.right_side.as_ref().map_or(false, |c| matches_card_type(&c.card))
                } else {
                    player.stage.total_blades() > 0
                }
            }
            "hand" => {
                if card_type_filter.is_some() {
                    player.hand.cards.iter().any(|c| matches_card_type(c))
                } else {
                    !player.hand.is_empty()
                }
            }
            "deck" => !player.main_deck.is_empty(),
            "discard" => {
                if card_type_filter.is_some() {
                    player.waitroom.cards.iter().any(|c| matches_card_type(c))
                } else {
                    !player.waitroom.cards.is_empty()
                }
            }
            "energy_zone" => !player.energy_zone.cards.is_empty(),
            "live_card_zone" => !player.live_card_zone.cards.is_empty(),
            "success_live_zone" => !player.success_live_card_zone.cards.is_empty(),
            _ => true,
        }
    }

    fn evaluate_position_condition(&self, condition: &Condition) -> bool {
        // Check position conditions (center, left_side, right_side)
        let target = condition.target.as_deref().unwrap_or("self");
        let position = condition.position.as_ref().and_then(|p| p.position.as_deref()).unwrap_or("");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        match position {
            "center" => player.stage.center.is_some(),
            "left_side" => player.stage.left_side.is_some(),
            "right_side" => player.stage.right_side.is_some(),
            "any" => player.stage.center.is_some() || player.stage.left_side.is_some() || player.stage.right_side.is_some(),
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
            "member_card" => player.stage.total_blades() as usize, // Approximate
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

    fn evaluate_appearance_condition(&self, _condition: &Condition) -> bool {
        // Check if a card has appeared (this would need to track appearance events)
        // For now, return true as a placeholder
        true
    }

    fn evaluate_temporal_condition(&self, _condition: &Condition) -> bool {
        // Check temporal conditions (this turn, live_end, etc.)
        // This would need to track turn state
        true // Placeholder
    }

    fn evaluate_state_condition(&self, condition: &Condition) -> bool {
        // Check card state (active, wait, face_up, face_down)
        let state = condition.state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        match state {
            "active" => {
                // Check if any stage cards are active
                player.stage.center.as_ref().map_or(false, |c| {
                    c.orientation.as_ref().map_or(false, |o| matches!(o, crate::zones::Orientation::Active))
                }) ||
                player.stage.left_side.as_ref().map_or(false, |c| {
                    c.orientation.as_ref().map_or(false, |o| matches!(o, crate::zones::Orientation::Active))
                }) ||
                player.stage.right_side.as_ref().map_or(false, |c| {
                    c.orientation.as_ref().map_or(false, |o| matches!(o, crate::zones::Orientation::Active))
                })
            }
            "wait" => {
                // Check if any stage cards are in wait state
                player.stage.center.as_ref().map_or(false, |c| {
                    c.orientation.as_ref().map_or(false, |o| matches!(o, crate::zones::Orientation::Wait))
                }) ||
                player.stage.left_side.as_ref().map_or(false, |c| {
                    c.orientation.as_ref().map_or(false, |o| matches!(o, crate::zones::Orientation::Wait))
                }) ||
                player.stage.right_side.as_ref().map_or(false, |c| {
                    c.orientation.as_ref().map_or(false, |o| matches!(o, crate::zones::Orientation::Wait))
                })
            }
            _ => true,
        }
    }

    fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        // Check energy card states
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

    fn evaluate_movement_condition(&self, _condition: &Condition) -> bool {
        // Check movement-related conditions
        true // Placeholder
    }

    fn evaluate_ability_negation_condition(&self, _condition: &Condition) -> bool {
        // Check if abilities are negated
        true // Placeholder
    }

    fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            conditions.iter().any(|c| self.evaluate_condition(c))
        } else {
            true
        }
    }

    fn evaluate_any_of_condition(&self, _condition: &Condition) -> bool {
        // Check if any condition is met
        true // Placeholder
    }

    fn evaluate_score_threshold_condition(&self, _condition: &Condition) -> bool {
        // Check score thresholds
        true // Placeholder
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
            "stage" => player.stage.total_blades(),
            "hand" => player.hand.len() as u32,
            "deck" => player.main_deck.len() as u32,
            "discard" => player.waitroom.len() as u32,
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => player.live_card_zone.len() as u32,
            "success_live_zone" => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    fn get_group_card_count(&self, _condition: &Condition) -> u32 {
        // Count cards of a specific group
        // This would need to iterate through cards and check their groups
        0 // Placeholder
    }

    /// Execute an ability effect
    pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // First, check if there's a condition
        if let Some(ref condition) = effect.condition {
            if !self.evaluate_condition(condition) {
                return Ok(()); // Condition not met, skip effect
            }
        }

        match effect.action.as_str() {
            "sequential" => self.execute_sequential_effect(effect),
            "conditional_alternative" => self.execute_conditional_alternative(effect),
            "look_and_select" => self.execute_look_and_select(effect),
            "draw" => self.execute_draw(effect),
            "move_cards" => self.execute_move_cards(effect),
            "gain_resource" => self.execute_gain_resource(effect),
            "change_state" => self.execute_change_state(effect),
            "modify_score" => self.execute_modify_score(effect),
            "modify_required_hearts" => self.execute_modify_required_hearts(effect),
            "set_cost" => self.execute_set_cost(effect),
            "set_blade_type" => self.execute_set_blade_type(effect),
            "set_heart_type" => self.execute_set_heart_type(effect),
            "activate_ability" => self.execute_activate_ability(effect),
            "invalidate_ability" => self.execute_invalidate_ability(effect),
            "play_baton_touch" => self.execute_play_baton_touch(effect),
            "reveal" => self.execute_reveal(effect),
            "select" => self.execute_select(effect),
            "look_at" => self.execute_look_at(effect),
            "modify_required_hearts_global" => self.execute_modify_required_hearts_global(effect),
            "modify_yell_count" => self.execute_modify_yell_count(effect),
            _ => {
                eprintln!("Unknown action type: {}", effect.action);
                Ok(())
            }
        }
    }

    fn execute_sequential_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        if let Some(ref actions) = effect.actions {
            for action in actions {
                self.execute_effect(action)?;
            }
        }
        Ok(())
    }

    fn execute_conditional_alternative(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Check alternative condition
        if let Some(ref alt_condition) = effect.alternative_condition {
            if self.evaluate_condition(alt_condition) {
                // Execute alternative effect
                if let Some(ref alt_effect) = effect.alternative_effect {
                    return self.execute_effect(alt_effect);
                }
            }
        }

        // Otherwise execute primary effect
        if let Some(ref primary_effect) = effect.primary_effect {
            self.execute_effect(primary_effect)
        } else {
            Ok(())
        }
    }

    fn execute_look_and_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Execute look action first
        if let Some(ref look_action) = effect.look_action {
            self.execute_effect(look_action)?;
        }

        // Then execute select action
        if let Some(ref select_action) = effect.select_action {
            self.execute_effect(select_action)?;
        }

        Ok(())
    }

    fn execute_draw(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        for _ in 0..count {
            if let Some(card) = player.main_deck.draw() {
                player.hand.add_card(card);
            }
        }

        Ok(())
    }

    fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let source = effect.source.as_deref().unwrap_or("");
        let destination = effect.destination.as_deref().unwrap_or("");
        let card_type_filter = effect.card_type.as_deref();
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Helper function to check if card matches type filter
        let matches_card_type = |card: &crate::card::Card, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card.is_live(),
                Some("member_card") => card.is_member(),
                Some("energy_card") => card.is_energy(),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        match source {
            "stage" => {
                // Move card from stage to destination
                let mut moved = 0;
                // For now, move from center position - would need position parameter
                if let Some(center_card) = player.stage.center.take() {
                    if matches_card_type(&center_card.card, card_type_filter) {
                        match destination {
                            "discard" => {
                                player.waitroom.add_card(center_card.card);
                                moved += 1;
                            }
                            "hand" => {
                                player.hand.add_card(center_card.card);
                                moved += 1;
                            }
                            _ => {
                                // Put card back
                                player.stage.center = Some(center_card);
                            }
                        }
                    } else {
                        // Put card back if it doesn't match
                        player.stage.center = Some(center_card);
                    }
                }
                // Also check left and right sides
                if moved < count {
                    if let Some(left_card) = player.stage.left_side.take() {
                        if matches_card_type(&left_card.card, card_type_filter) {
                            match destination {
                                "discard" => {
                                    player.waitroom.add_card(left_card.card);
                                    moved += 1;
                                }
                                "hand" => {
                                    player.hand.add_card(left_card.card);
                                    moved += 1;
                                }
                                _ => {
                                    player.stage.left_side = Some(left_card);
                                }
                            }
                        } else {
                            player.stage.left_side = Some(left_card);
                        }
                    }
                }
                if moved < count {
                    if let Some(right_card) = player.stage.right_side.take() {
                        if matches_card_type(&right_card.card, card_type_filter) {
                            match destination {
                                "discard" => {
                                    player.waitroom.add_card(right_card.card);
                                    moved += 1;
                                }
                                "hand" => {
                                    player.hand.add_card(right_card.card);
                                    moved += 1;
                                }
                                _ => {
                                    player.stage.right_side = Some(right_card);
                                }
                            }
                        } else {
                            player.stage.right_side = Some(right_card);
                        }
                    }
                }
            }
            "deck" | "deck_top" => {
                let mut moved = 0;
                while moved < count {
                    if let Some(card) = player.main_deck.draw() {
                        if matches_card_type(&card, card_type_filter) {
                            match destination {
                                "hand" => player.hand.add_card(card),
                                "discard" => player.waitroom.add_card(card),
                                _ => {}
                            }
                            moved += 1;
                        } else {
                            // Card doesn't match filter, put it back
                            player.main_deck.cards.push_back(card);
                            break; // Stop if we encounter non-matching card
                        }
                    } else {
                        break; // Deck empty
                    }
                }
            }
            "hand" => {
                match destination {
                    "discard" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(card, card_type_filter) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            player.waitroom.add_card(card);
                        }
                    }
                    _ => {}
                }
            }
            "discard" => {
                match destination {
                    "hand" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(card, card_type_filter) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            player.hand.add_card(card);
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                eprintln!("Unknown source for move_cards: {}", source);
            }
        }

        Ok(())
    }

    fn execute_gain_resource(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let resource = effect.resource.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let _duration = effect.duration.as_deref(); // TODO: Handle duration (live_end, etc.)

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        match resource {
            "blade" => {
                // Add blades to all stage cards
                if let Some(ref mut left) = player.stage.left_side {
                    left.card.add_blades(count);
                }
                if let Some(ref mut center) = player.stage.center {
                    center.card.add_blades(count);
                }
                if let Some(ref mut right) = player.stage.right_side {
                    right.card.add_blades(count);
                }
            }
            "heart" => {
                // Add hearts to all stage cards
                // For now, add heart01 as default - would need heart_color parameter
                if let Some(ref mut left) = player.stage.left_side {
                    left.card.add_heart("heart01", count);
                }
                if let Some(ref mut center) = player.stage.center {
                    center.card.add_heart("heart01", count);
                }
                if let Some(ref mut right) = player.stage.right_side {
                    right.card.add_heart("heart01", count);
                }
            }
            "energy" => {
                // Add energy cards to energy zone
                // This would need to actually create/add energy cards
                eprintln!("Gain resource: energy (not fully implemented)");
            }
            _ => {
                eprintln!("Unknown resource type: {}", resource);
            }
        }

        Ok(())
    }

    fn execute_change_state(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let state_change = effect.state_change.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);

        let player = &mut self.game_state.player1;

        match state_change {
            "active" => {
                if count == 2 {
                    player.energy_zone.activate_all();
                } else {
                    player.stage.activate_all_cards();
                }
            }
            "wait" => {
                player.stage.set_all_orientation(crate::zones::Orientation::Wait);
            }
            _ => {}
        }

        Ok(())
    }

    fn execute_modify_score(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // For now, modify score on all stage cards
        // A full implementation would need to target specific cards based on position/condition
        match operation {
            "add" => {
                if let Some(ref mut left) = player.stage.left_side {
                    left.card.add_score(value);
                }
                if let Some(ref mut center) = player.stage.center {
                    center.card.add_score(value);
                }
                if let Some(ref mut right) = player.stage.right_side {
                    right.card.add_score(value);
                }
            }
            "remove" => {
                if let Some(ref mut left) = player.stage.left_side {
                    left.card.remove_score(value);
                }
                if let Some(ref mut center) = player.stage.center {
                    center.card.remove_score(value);
                }
                if let Some(ref mut right) = player.stage.right_side {
                    right.card.remove_score(value);
                }
            }
            "set" => {
                if let Some(ref mut left) = player.stage.left_side {
                    left.card.set_score(value);
                }
                if let Some(ref mut center) = player.stage.center {
                    center.card.set_score(value);
                }
                if let Some(ref mut right) = player.stage.right_side {
                    right.card.set_score(value);
                }
            }
            _ => {
                eprintln!("Unknown modify_score operation: {}", operation);
            }
        }

        Ok(())
    }

    fn execute_modify_required_hearts(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("decrease");
        let _value = effect.value.unwrap_or(0);

        // This would need to target specific live cards
        match operation {
            "decrease" => {
                // Decrease required hearts
            }
            "increase" => {
                // Increase required hearts
            }
            _ => {}
        }

        Ok(())
    }

    fn execute_set_cost(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Set card cost
        Ok(())
    }

    fn execute_set_blade_type(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Set blade type
        Ok(())
    }

    fn execute_set_heart_type(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Set heart type
        Ok(())
    }

    fn execute_activate_ability(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Activate an ability
        Ok(())
    }

    fn execute_invalidate_ability(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Invalidate an ability
        Ok(())
    }

    fn execute_play_baton_touch(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Execute baton touch
        Ok(())
    }

    fn execute_reveal(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Reveal cards from a location to the opponent
        // For now, this is a placeholder - would need to track revealed state
        Ok(())
    }

    fn execute_select(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Select cards from a location (typically after looking)
        // For now, this is a placeholder - would need UI integration for selection
        Ok(())
    }

    fn execute_look_at(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Look at cards from a location without revealing
        // For now, this is a placeholder - would need to track looked-at state
        Ok(())
    }

    fn execute_modify_required_hearts_global(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Modify required hearts for all live cards in a zone
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let _value = effect.value.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // For now, just log the operation - would need target_location field
        eprintln!("Modify required hearts globally for player: {}", operation);
        Ok(())
    }

    fn execute_modify_yell_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Modify yell count (cheer blade/heart count)
        let operation = effect.operation.as_deref().unwrap_or("subtract");
        let count = effect.count.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let _player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        // This would need to modify the cheer blade/heart count
        // For now, just log the operation
        match operation {
            "add" => {
                eprintln!("Add {} to yell count for player", count);
            }
            "subtract" => {
                eprintln!("Subtract {} from yell count for player", count);
            }
            _ => {
                eprintln!("Unknown modify_yell_count operation: {}", operation);
            }
        }

        Ok(())
    }

    /// Resolve a complete ability
    pub fn resolve_ability(&mut self, ability: &Ability) -> Result<(), String> {
        // First, pay the cost if there is one
        if let Some(ref cost) = ability.cost {
            self.pay_cost(cost)?;
        }

        // Then execute the effect
        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect)?;
        }

        Ok(())
    }

    fn pay_cost(&mut self, cost: &AbilityCost) -> Result<(), String> {
        eprintln!("PAY_COST: cost_type={:?}, source={:?}, destination={:?}, card_type={:?}", cost.cost_type, cost.source, cost.destination, cost.card_type);
        match cost.cost_type.as_deref() {
            Some("move_cards") => {
                // Execute the move action as a cost
                let effect = AbilityEffect {
                    text: cost.text.clone(),
                    action: cost.cost_type.clone().unwrap_or_default(),
                    source: cost.source.clone(),
                    destination: cost.destination.clone(),
                    count: cost.count,
                    card_type: cost.card_type.clone(),
                    target: cost.target.clone(),
                    ..Default::default()
                };
                self.execute_move_cards(&effect)
            }
            Some("pay_energy") => {
                // Pay energy cost
                let energy = cost.energy.unwrap_or(0);
                if energy > 0 {
                    // Deactivate energy cards
                    self.game_state.player1.energy_zone.cards.iter_mut().take(energy as usize).for_each(|card| {
                        card.orientation = Some(crate::zones::Orientation::Wait);
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

// Implement Default for AbilityEffect
impl Default for AbilityEffect {
    fn default() -> Self {
        AbilityEffect {
            heart_color: None,
            blade_type: None,
            energy_count: None,
            target_member: None,
            text: String::new(),
            action: String::new(),
            source: None,
            destination: None,
            count: None,
            card_type: None,
            target: None,
            duration: None,
            parenthetical: None,
            look_action: None,
            select_action: None,
            actions: None,
            resource: None,
            position: None,
            state_change: None,
            optional: None,
            max: None,
            effect_constraint: None,
            shuffle_target: None,
            icon_count: None,
            ability_gain: None,
            quoted_text: None,
            per_unit: None,
            condition: None,
            primary_effect: None,
            alternative_condition: None,
            alternative_effect: None,
            operation: None,
            value: None,
            aggregate: None,
            comparison_type: None,
            choice_options: None,
            group: None,
            per_unit_count: None,
            per_unit_type: None,
            per_unit_reference: None,
            group_matching: None,
            repeat_limit: None,
            repeat_optional: None,
            is_further: None,
            cost_result_reference: None,
            dynamic_count: None,
            resource_icon_count: None,
            placement_order: None,
            cost_limit: None,
            unit: None,
            distinct: None,
            target_player: None,
            target_location: None,
            target_scope: None,
            target_card_type: None,
            activation_condition: None,
            activation_condition_parsed: None,
            gained_ability: None,
            ability_text: None,
            swap_action: None,
            has_member_swapping: None,
            group_options: None,
            card_count: None,
            use_limit: None,
            triggers: None,
        }
    }
}
