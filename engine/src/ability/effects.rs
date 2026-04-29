use crate::card::{AbilityCost, AbilityEffect};
use crate::game_state::GameState;
use crate::player::Player;
use crate::zones::MemberArea;
use std::sync::Arc;
use super::condition::evaluate_condition;

#[derive(Debug, Clone)]
pub struct EffectExecutor {
    pub looked_at_cards: Vec<u32>,
    pub selected_cards: Vec<u32>,
}

impl EffectExecutor {
    pub fn new() -> Self {
        Self {
            looked_at_cards: Vec::new(),
            selected_cards: Vec::new(),
        }
    }

    /// Execute a move_cards effect
    pub fn execute_move_cards(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("");
        let destination = effect.destination.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);
        let card_type = effect.card_type.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);
        let target_player_ids: Vec<String> = target_players
            .into_iter()
            .filter(|tp| tp.id == player.id)
            .map(|tp| tp.id.clone())
            .collect();

        if !target_player_ids.is_empty() {
            let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
            match (source, destination) {
                ("discard", "hand") => {
                    self.move_from_discard_to_hand(player, count_usize, card_type, game_state)?;
                }
                ("stage", "discard") => {
                    self.move_from_stage_to_discard(player, false, false, game_state)?;
                }
                ("hand", "discard") => {
                    self.move_from_hand_to_discard(player, count_usize)?;
                }
                ("deck", "hand") => {
                    self.draw_cards(player, count_usize)?;
                }
                ("deck_top", "hand") => {
                    self.draw_cards(player, count_usize)?;
                }
                ("hand", "deck_bottom") => {
                    self.move_from_hand_to_deck_bottom(player, count_usize)?;
                }
                ("looked_at", "deck_top") => {
                    self.move_from_looked_at_to_deck_top(player, count_usize)?;
                }
                ("looked_at_remaining", "discard") => {
                    self.move_looked_at_remaining_to_discard(player)?;
                }
                ("selected_cards", "hand") => {
                    self.move_from_selected_to_hand(player, count_usize)?;
                }
                _ => {
                    return Err(format!(
                        "Unsupported move: {} -> {}",
                        source, destination
                    ));
                }
            }
        }

        Ok(())
    }

    fn move_from_discard_to_hand(
        &mut self,
        player: &mut Player,
        count: usize,
        card_type: &str,
        game_state: &GameState,
    ) -> Result<(), String> {
        let card_db = &game_state.card_database;
        let matching_indices: Vec<usize> = player.waitroom.cards.iter().enumerate()
            .filter(|(_, card_id)| {
                if let Some(card) = card_db.get_card(**card_id) {
                    match card_type {
                        "member_card" => card.is_member(),
                        "live_card" => card.is_live(),
                        _ => true,
                    }
                } else {
                    false
                }
            })
            .map(|(i, _)| i)
            .collect();

        if matching_indices.len() < count {
            return Err(format!(
                "Not enough cards in discard: needed {}, have {}",
                count, matching_indices.len()
            ));
        }

        if matching_indices.len() > count {
            return Err("Pending choice required: select cards from discard to add to hand".to_string());
        }

        let mut moved = 0;
        let mut indices_to_remove = Vec::new();

        for (i, card_id) in player.waitroom.cards.iter().enumerate() {
            if moved >= count {
                break;
            }

            let matches_type = if let Some(card) = card_db.get_card(*card_id) {
                match card_type {
                    "member_card" => card.is_member(),
                    "live_card" => card.is_live(),
                    _ => true,
                }
            } else {
                false
            };

            if matches_type {
                indices_to_remove.push(i);
                player.hand.add_card(*card_id);
                moved += 1;
            }
        }

        for i in indices_to_remove.into_iter().rev() {
            player.waitroom.cards.remove(i);
        }

        Ok(())
    }

    fn move_from_stage_to_discard(&mut self, player: &mut Player, is_self_cost: bool, exclude_self: bool, game_state: &GameState) -> Result<(), String> {
        if is_self_cost {
            if let Some(activating_card_id) = game_state.activating_card {
                let mut found = false;
                for i in 0..3 {
                    if player.stage.stage[i] == activating_card_id {
                        player.stage.stage[i] = -1;
                        player.waitroom.add_card(activating_card_id);
                        eprintln!("Self-cost: discarded activating card {} from stage position {}", activating_card_id, i);
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(format!("Activating card {} not found on stage", activating_card_id));
                }
            } else {
                let stage_count = player.stage.stage.iter().filter(|&&c| c != -1).count();
                if stage_count == 0 {
                    return Err("No cards on stage to discard".to_string());
                }
                eprintln!("Self-cost: no activating card tracked, removing first card");
                for i in 0..3 {
                    if player.stage.stage[i] != -1 {
                        let card_id = player.stage.stage[i];
                        player.stage.stage[i] = -1;
                        player.waitroom.add_card(card_id);
                        eprintln!("Discarded card at position {}", i);
                        break;
                    }
                }
            }
        } else {
            let activating_card_id = game_state.activating_card;
            let mut cards_to_discard = Vec::new();
            
            for i in 0..3 {
                if player.stage.stage[i] != -1 {
                    let card_id = player.stage.stage[i];
                    if exclude_self && activating_card_id == Some(card_id) {
                        eprintln!("Excluding activating card {} from discard", card_id);
                        continue;
                    }
                    cards_to_discard.push((i, card_id));
                }
            }
            
            if cards_to_discard.is_empty() {
                return Err("No cards on stage to discard (after exclude_self filter)".to_string());
            }
            
            if cards_to_discard.len() > 1 {
                return Err("Pending choice required: select card to discard from stage".to_string());
            }
            
            for (i, card_id) in cards_to_discard {
                player.stage.stage[i] = -1;
                player.waitroom.add_card(card_id);
                eprintln!("Discarded card {} from stage position {}", card_id, i);
            }
        }
        Ok(())
    }

    fn move_from_hand_to_discard(&mut self, player: &mut Player, count: usize) -> Result<(), String> {
        if player.hand.cards.len() > count {
            return Err("Pending choice required: select cards to discard from hand".to_string());
        }
        
        let cards_to_remove: Vec<_> = player.hand.cards.iter().take(count).copied().collect();
        for card_id in cards_to_remove {
            player.waitroom.add_card(card_id);
        }
        let remove_count = count.min(player.hand.cards.len());
        player.hand.cards.drain(..remove_count);
        Ok(())
    }

    fn discard_until_count(&mut self, player: &mut Player, target_count: usize) -> Result<(), String> {
        let current_count = player.hand.cards.len();
        if current_count <= target_count {
            return Ok(());
        }
        
        let cards_to_discard = current_count - target_count;
        if cards_to_discard > 0 {
            return Err("Pending choice required: select cards to discard from hand".to_string());
        }
        
        Ok(())
    }

    fn draw_cards(&self, player: &mut Player, count: usize) -> Result<(), String> {
        for _ in 0..count {
            if let Some(card_id) = player.main_deck.draw() {
                player.hand.add_card(card_id);
            } else {
                return Err("Deck is empty".to_string());
            }
        }
        Ok(())
    }

    fn move_from_hand_to_deck_bottom(&mut self, player: &mut Player, count: usize) -> Result<(), String> {
        if player.hand.cards.len() > count {
            return Err("Pending choice required: select cards to move from hand to deck bottom".to_string());
        }

        let cards_to_move: Vec<_> = player.hand.cards.iter().take(count).copied().collect();
        for card_id in cards_to_move {
            player.main_deck.cards.push(card_id);
        }
        let remove_count = count.min(player.hand.cards.len());
        player.hand.cards.drain(..remove_count);
        Ok(())
    }

    fn move_from_looked_at_to_deck_top(
        &mut self,
        player: &mut Player,
        count: usize,
    ) -> Result<(), String> {
        if self.looked_at_cards.is_empty() {
            return Err("No looked-at cards available".to_string());
        }

        let actual_count = count.min(self.looked_at_cards.len());
        let cards_to_move: Vec<u32> = self.looked_at_cards.drain(..actual_count).collect();
        
        for card_id in cards_to_move.into_iter().rev() {
            player.main_deck.cards.insert(0, card_id as i16);
        }

        eprintln!("Moved {} looked-at cards to deck top (requested: {})", actual_count, count);
        Ok(())
    }

    fn move_looked_at_remaining_to_discard(
        &mut self,
        player: &mut Player,
    ) -> Result<(), String> {
        let remaining_count = self.looked_at_cards.len();
        if remaining_count > 0 {
            let cards_to_move: Vec<i16> = self.looked_at_cards.drain(..)
                .map(|id| id.try_into().unwrap())
                .collect();
            for card_id in cards_to_move {
                player.waitroom.add_card(card_id);
            }
            eprintln!("Moved {} remaining looked-at cards to discard", remaining_count);
        }
        Ok(())
    }

    fn move_from_selected_to_hand(
        &mut self,
        player: &mut Player,
        count: usize,
    ) -> Result<(), String> {
        if self.selected_cards.is_empty() {
            return Err("No selected cards available".to_string());
        }

        let cards_to_move: Vec<i16> = self.selected_cards.drain(..)
            .take(count)
            .map(|id| id.try_into().unwrap())
            .collect();
        for card_id in cards_to_move {
            player.hand.add_card(card_id);
        }
        Ok(())
    }

    /// Execute a draw effect
    pub fn execute_draw(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1) as usize;
        self.draw_cards(player, count)
    }

    /// Execute a gain_resource effect (blades)
    pub fn execute_gain_resource(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let resource = effect.resource.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1);

        if resource != "blade" && resource != "ブレード" && resource != "heart" && resource != "ハート" {
            return Err(format!("Unsupported resource: {}", resource));
        }

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut blade_modifications: Vec<(i16, i32)> = Vec::new();
        let mut heart_modifications: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();

        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            let areas = [0, 1, 2];
            for index in areas {
                let card_id = target_player.stage.stage[index];
                if card_id != -1 {
                    match resource {
                        "blade" | "ブレード" => {
                            blade_modifications.push((card_id, count as i32));
                        }
                        "heart" | "ハート" => {
                            let color = if let Some(ref heart_color) = effect.heart_color {
                                crate::zones::parse_heart_color(heart_color)
                            } else {
                                crate::card::HeartColor::Heart00
                            };
                            heart_modifications.push((card_id, color, count as i32));
                        }
                        _ => {}
                    }
                }
            }
        }

        for (card_id, modifier) in blade_modifications {
            game_state.add_blade_modifier(card_id, modifier);
        }
        for (card_id, color, modifier) in heart_modifications {
            game_state.add_heart_modifier(card_id, color, modifier);
        }

        Ok(())
    }

    /// Execute a modify_score effect
    pub fn execute_modify_score(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<(i16, i32)> = Vec::new();
        
        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            for card_id in &target_player.live_card_zone.cards {
                match operation {
                    "add" => {
                        card_ids_to_modify.push((*card_id, value as i32));
                    }
                    "remove" => {
                        card_ids_to_modify.push((*card_id, -(value as i32)));
                    }
                    "set" => {
                        card_ids_to_modify.push((*card_id, value as i32));
                    }
                    _ => return Err(format!("Unknown operation: {}", operation)),
                }
            }
        }
        
        for (card_id, delta) in card_ids_to_modify {
            if operation == "set" {
                game_state.score_modifiers.insert(card_id, delta);
            } else {
                game_state.add_score_modifier(card_id, delta);
            }
        }

        Ok(())
    }

    /// Execute a modify_required_hearts effect
    pub fn execute_modify_required_hearts(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("decrease");
        let value = effect.value.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();
        
        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            let color = crate::zones::parse_heart_color(heart_color);
            for card_id in &target_player.live_card_zone.cards {
                match operation {
                    "decrease" => {
                        card_ids_to_modify.push((*card_id, color, -(value as i32)));
                    }
                    "increase" => {
                        card_ids_to_modify.push((*card_id, color, value as i32));
                    }
                    "set" => {
                        card_ids_to_modify.push((*card_id, color, value as i32));
                    }
                    _ => return Err(format!("Unknown operation: {}", operation)),
                }
            }
        }
        
        for (card_id, color, delta) in card_ids_to_modify {
            if operation == "set" {
                game_state.set_need_heart_modifier(card_id, color, delta);
            } else {
                game_state.add_need_heart_modifier(card_id, color, delta);
            }
        }

        Ok(())
    }

    /// Execute a set_required_hearts effect
    pub fn execute_set_required_hearts(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();
        
        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            let color = crate::zones::parse_heart_color(heart_color);
            for card_id in &target_player.live_card_zone.cards {
                card_ids_to_modify.push((*card_id, color, count as i32));
            }
        }
        
        for (card_id, color, count) in card_ids_to_modify {
            game_state.set_need_heart_modifier(card_id, color, count);
        }

        Ok(())
    }

    /// Execute a modify_required_hearts_global effect
    pub fn execute_modify_required_hearts_global(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let value = effect.value.unwrap_or(1);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("opponent");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();

        for target_player in target_players {
            let color = crate::zones::parse_heart_color(heart_color);
            for card_id in &target_player.live_card_zone.cards {
                let modifier_value = match operation {
                    "increase" => value as i32,
                    "decrease" => -(value as i32),
                    _ => return Err(format!("Unknown operation: {}", operation)),
                };
                card_ids_to_modify.push((*card_id, color, modifier_value));
            }
        }

        for (card_id, color, modifier_value) in card_ids_to_modify {
            game_state.add_need_heart_modifier(card_id, color, modifier_value);
        }

        Ok(())
    }

    /// Execute a set_blade_type effect
    pub fn execute_set_blade_type(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let blade_type = effect.blade_type.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        let current_turn = game_state.turn_number;
        let current_phase = game_state.current_phase.clone();
        let effect_duration = effect.duration.clone();
        let card_db = game_state.card_database.clone();

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);
        
        let mut temp_effects = Vec::new();

        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            let areas = [0, 1, 2];
            for index in areas {
                let card_id = target_player.stage.stage[index];
                if card_id != -1 {
                    let temp_effect = crate::game_state::TemporaryEffect {
                        effect_type: format!("set_blade_type:{}", blade_type),
                        duration: effect_duration.as_ref().map(|d| match d.as_str() {
                            "live_end" => crate::game_state::Duration::LiveEnd,
                            "this_turn" => crate::game_state::Duration::ThisTurn,
                            "this_live" => crate::game_state::Duration::ThisLive,
                            "permanent" => crate::game_state::Duration::Permanent,
                            "as_long_as" => crate::game_state::Duration::ThisLive,
                            _ => crate::game_state::Duration::ThisLive,
                        }).unwrap_or(crate::game_state::Duration::ThisLive),
                        created_turn: current_turn,
                        created_phase: current_phase.clone(),
                        target_player_id: target_player.id.clone(),
                        description: format!("Set blade type to {} for {}", blade_type, card_db.get_card(card_id).map(|c| c.name.as_str()).unwrap_or("unknown")),
                        creation_order: 0,
                        effect_data: None,
                    };
                    temp_effects.push(temp_effect);
                }
            }
        }
        
        for effect in temp_effects {
            game_state.temporary_effects.push(effect);
        }

        Ok(())
    }

    /// Execute a set_heart_type effect
    pub fn execute_set_heart_type(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let heart_type = effect.heart_type.as_deref().or(effect.heart_color.as_deref()).unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1) as i32;

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<i16> = Vec::new();

        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            let areas = [0, 1, 2];
            for index in areas {
                let card_id = target_player.stage.stage[index];
                if card_id != -1 {
                    card_ids_to_modify.push(card_id);
                }
            }
        }

        let color = crate::zones::parse_heart_color(heart_type);
        for card_id in card_ids_to_modify {
            game_state.add_heart_modifier(card_id, color, count);
            eprintln!("Added heart modifier: card_id={}, color={:?}, count={}", card_id, color, count);
        }

        Ok(())
    }

    /// Execute a position_change effect
    pub fn execute_position_change(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let position = effect.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");

        let card_database = Arc::clone(&game_state.card_database);
        let target_players = game_state.resolve_target_mut(target, perspective_player_id);
        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            let target_index = match position {
                "center" | "センターエリア" => 1,
                "left_side" | "左サイドエリア" => 0,
                "right_side" | "右サイドエリア" => 2,
                _ => return Err(format!("Unknown position: {}", position)),
            };

            let current_index = if target_member == "this_member" {
                return Err("position_change with 'this_member' requires context tracking - not yet implemented".to_string());
            } else {
                target_player.stage.stage.iter()
                    .position(|&card_id| {
                        if card_id == -1 {
                            false
                        } else {
                            card_database.get_card(card_id)
                                .map(|c| c.card_no == target_member)
                                .unwrap_or(false)
                        }
                    })
            };

            if let Some(current_idx) = current_index {
                let card_id = target_player.stage.stage[current_idx];
                
                if target_player.stage.stage[target_index] != -1 {
                    let occupying_card = target_player.stage.stage[target_index];
                    target_player.stage.stage[target_index] = card_id;
                    target_player.stage.stage[current_idx] = occupying_card;
                    println!("Position change: swapped members between index {} and {}", current_idx, target_index);
                } else {
                    target_player.stage.stage[target_index] = card_id;
                    target_player.stage.stage[current_idx] = -1;
                    println!("Position change: moved member from index {} to {}", current_idx, target_index);
                }
            } else {
                return Err(format!("Member not found: {}", target_member));
            }
        }

        Ok(())
    }

    /// Execute a place_energy_under_member effect
    pub fn execute_place_energy_under_member(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let energy_count = effect.energy_count.unwrap_or(1);
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");
        let target = effect.target.as_deref().unwrap_or("self");

        let activating_card_id = game_state.activating_card;
        
        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        for target_player in target_players {
            if target_player.id != player.id {
                continue;
            }

            for _ in 0..energy_count {
                if let Some(energy_card) = target_player.energy_deck.draw() {
                    match target_member {
                        "this_member" => {
                            if let Some(activating_id) = activating_card_id {
                                let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                                let mut placed = false;
                                for area in &areas {
                                    if let Some(stage_card_id) = target_player.stage.get_area(*area) {
                                        if stage_card_id == activating_id {
                                            target_player.energy_zone.cards.push(energy_card);
                                            eprintln!("Placed energy {} under member {} at {:?}", 
                                                     energy_card, activating_id, area);
                                            placed = true;
                                            break;
                                        }
                                    }
                                }
                                if !placed {
                                    target_player.energy_zone.cards.push(energy_card);
                                }
                            } else {
                                target_player.energy_zone.cards.push(energy_card);
                            }
                        }
                        _ => {
                            let target_area = match target_member {
                                "left" | "left_side" => Some(crate::zones::MemberArea::LeftSide),
                                "center" => Some(crate::zones::MemberArea::Center),
                                "right" | "right_side" => Some(crate::zones::MemberArea::RightSide),
                                _ => None,
                            };
                            
                            if let Some(area) = target_area {
                                if let Some(member_id) = target_player.stage.get_area(area) {
                                    target_player.energy_zone.cards.push(energy_card);
                                    eprintln!("Placed energy {} under member {} at {:?}", 
                                             energy_card, member_id, area);
                                } else {
                                    target_player.energy_zone.cards.push(energy_card);
                                }
                            } else {
                                target_player.energy_zone.cards.push(energy_card);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a modify_yell_count effect
    pub fn execute_modify_yell_count(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("subtract");
        let count = effect.count.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let should_apply = target_players.iter().any(|p| p.id == player.id);

        if should_apply {
            match operation {
                "add" => {
                    game_state.cheer_checks_required += count;
                }
                "subtract" => {
                    game_state.cheer_checks_required = game_state.cheer_checks_required.saturating_sub(count);
                }
                "set" => {
                    game_state.cheer_checks_required = count;
                }
                _ => return Err(format!("Unknown operation: {}", operation)),
            }
        }

        Ok(())
    }

    /// Execute a look_at effect (look at top cards of deck without moving)
    pub fn execute_look_at(
        &mut self,
        effect: &AbilityEffect,
        player: &Player,
    ) -> Result<Vec<i16>, String> {
        let count = effect.count.unwrap_or(1);
        let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
        let cards = player.main_deck.peek_top(count_usize);

        if cards.len() < count_usize {
            return Err(format!(
                "Not enough cards in deck: needed {}, have {}",
                count_usize,
                cards.len()
            ));
        }

        Ok(cards)
    }

    /// Execute sequential actions (multiple effects in order)
    pub fn execute_sequential(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let actions = effect.actions.as_ref().ok_or("No actions in sequential effect")?;
        let is_conditional = effect.conditional.unwrap_or(false);
        let condition = effect.condition.as_ref();

        if is_conditional {
            if let Some(cond) = condition {
                let condition_met = evaluate_condition(cond, player, game_state);
                eprintln!("Conditional sequential effect with condition: {}, met: {}", cond.text, condition_met);
                
                if !condition_met {
                    eprintln!("Condition not met, skipping sequential actions");
                    return Ok(());
                }
            }
        }

        for (index, sub_effect) in actions.iter().enumerate() {
            match sub_effect.action.as_str() {
                "draw" | "draw_card" => {
                    self.execute_draw(sub_effect, player)?;
                }
                "move_cards" => {
                    match self.execute_move_cards(sub_effect, player, game_state, perspective_player_id) {
                        Ok(_) => {},
                        Err(e) if e.contains("Pending choice required") => {
                            let remaining_actions: Vec<AbilityEffect> = actions[index + 1..].to_vec();
                            if !remaining_actions.is_empty() {
                                game_state.pending_sequential_actions = Some(remaining_actions);
                            }
                            return Err(e);
                        }
                        Err(e) => return Err(e),
                    }
                }
                "look_at" => {
                    self.execute_look_at(sub_effect, player)?;
                }
                "gain_resource" => {
                    self.execute_gain_resource(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_score" => {
                    self.execute_modify_score(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_required_hearts" => {
                    self.execute_modify_required_hearts(sub_effect, player, game_state, perspective_player_id)?;
                }
                "set_required_hearts" => {
                    self.execute_set_required_hearts(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_required_hearts_global" => {
                    self.execute_modify_required_hearts_global(sub_effect, player, game_state, perspective_player_id)?;
                }
                "set_blade_type" => {
                    self.execute_set_blade_type(sub_effect, player, game_state, perspective_player_id)?;
                }
                "set_heart_type" => {
                    self.execute_set_heart_type(sub_effect, player, game_state, perspective_player_id)?;
                }
                "position_change" => {
                    self.execute_position_change(sub_effect, player, game_state, perspective_player_id)?;
                }
                "place_energy_under_member" => {
                    self.execute_place_energy_under_member(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_yell_count" => {
                    self.execute_modify_yell_count(sub_effect, player, game_state, perspective_player_id)?;
                }
                _ => {
                    return Err(format!("Unknown action in sequence: {}", sub_effect.action));
                }
            }
        }

        Ok(())
    }

    /// Execute an ability effect (dispatch to appropriate handler)
    pub fn execute_effect(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        match effect.action.as_str() {
            "move_cards" => {
                self.execute_move_cards(effect, player, game_state, perspective_player_id)
            }
            "draw" | "draw_card" => self.execute_draw(effect, player),
            "discard_until_count" => {
                let target_count = effect.target_count.unwrap_or(0) as usize;
                self.discard_until_count(player, target_count)
            }
            "gain_resource" => {
                self.execute_gain_resource(effect, player, game_state, perspective_player_id)
            }
            "modify_score" => {
                self.execute_modify_score(effect, player, game_state, perspective_player_id)
            }
            "modify_required_hearts" => {
                self.execute_modify_required_hearts(effect, player, game_state, perspective_player_id)
            }
            "set_required_hearts" => {
                self.execute_set_required_hearts(effect, player, game_state, perspective_player_id)
            }
            "modify_required_hearts_global" => {
                self.execute_modify_required_hearts_global(effect, player, game_state, perspective_player_id)
            }
            "set_blade_type" => {
                self.execute_set_blade_type(effect, player, game_state, perspective_player_id)
            }
            "set_heart_type" => {
                self.execute_set_heart_type(effect, player, game_state, perspective_player_id)
            }
            "position_change" => {
                self.execute_position_change(effect, player, game_state, perspective_player_id)
            }
            "place_energy_under_member" => {
                self.execute_place_energy_under_member(effect, player, game_state, perspective_player_id)
            }
            "modify_yell_count" => {
                self.execute_modify_yell_count(effect, player, game_state, perspective_player_id)
            }
            "look_at" => {
                self.execute_look_at(effect, player)?;
                Ok(())
            }
            "sequential" => {
                self.execute_sequential(effect, player, game_state, perspective_player_id)
            }
            _ => Err(format!("Unknown effect action: {}", effect.action)),
        }
    }

    /// Execute ability cost
    pub fn execute_cost(
        &mut self,
        cost: &AbilityCost,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs {
                        self.execute_cost(sub_cost, player, game_state, perspective_player_id)?;
                    }
                }
                Ok(())
            }
            Some("move_cards") => {
                let source = cost.source.as_deref().unwrap_or("");
                let destination = cost.destination.as_deref().unwrap_or("");

                match (source, destination) {
                    ("stage" | "ステージ", "discard" | "控え室") => {
                        let is_self_cost = cost.self_cost.unwrap_or(false);
                        let exclude_self = cost.exclude_self.unwrap_or(false);
                        self.move_from_stage_to_discard(player, is_self_cost, exclude_self, game_state)?;
                    }
                    ("hand" | "手札", "discard" | "控え室") => {
                        let count = cost.count.unwrap_or(1);
                        let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
                        self.move_from_hand_to_discard(player, count_usize)?;
                    }
                    ("hand" | "手札", "deck_bottom") => {
                        let count = cost.count.unwrap_or(1);
                        let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
                        self.move_from_hand_to_deck_bottom(player, count_usize)?;
                    }
                    _ => {
                        return Err(format!(
                            "Unsupported cost move: {} -> {}",
                            source, destination
                        ));
                    }
                }
                Ok(())
            }
            Some("pay_energy") => {
                let energy_needed = cost.energy.unwrap_or(1) as usize;
                let deactivated = energy_needed;

                if deactivated < energy_needed {
                    return Err(format!(
                        "Could not pay energy: needed {}, deactivated {}",
                        energy_needed, deactivated
                    ));
                }
                Ok(())
            }
            Some("change_state") => {
                let state = cost.state_change.as_deref().unwrap_or("");
                let position = cost.position.as_ref().and_then(|p| p.get_position());

                if let Some(pos) = position {
                    let _area = match pos {
                        "center" | "センターエリア" => MemberArea::Center,
                        "left_side" | "左サイドエリア" => MemberArea::LeftSide,
                        "right_side" | "右サイドエリア" => MemberArea::RightSide,
                        _ => return Err(format!("Unknown position: {}", pos)),
                    };

                    let _orientation = match state {
                        "active" | "アクティブ" => crate::zones::Orientation::Active,
                        "wait" | "ウェイト" => crate::zones::Orientation::Wait,
                        _ => return Err(format!("Unknown state: {}", state)),
                    };
                }
                Ok(())
            }
            Some("reveal") => {
                eprintln!("Reveal cost: {}", cost.text);
                Ok(())
            }
            _ => Err(format!("Unknown cost type: {:?}", cost.cost_type)),
        }
    }
}
