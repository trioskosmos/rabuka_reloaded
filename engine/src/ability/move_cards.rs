use crate::card::AbilityEffect;
use super::types::{Choice, ExecutionContext};
use super::resolver::AbilityResolver;

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    pub fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let _max = effect.max.unwrap_or(false);
        let source = effect.source.as_deref().unwrap_or("").to_string();
        let destination = effect.destination.as_deref().unwrap_or("").to_string();
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();

        if let Some(ref constraint) = effect.effect_constraint {
            eprintln!("Effect constraint: {}", constraint);
        }

        if let Some(ref position_info) = effect.position {
            if let Some(deck_pos) = position_info.get_position() {
                eprintln!("Deck position: {}", deck_pos);
            }
        }

        let _optional = effect.optional.unwrap_or(false);
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let cost_limit = effect.cost_limit;

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let card_db = self.game_state.card_database.clone();

        let source_card_count = match source.as_str() {
            "stage" => player.stage.stage.iter().filter(|&&x| x != -1).count(),
            "deck" | "deck_top" => player.main_deck.cards.len(),
            "hand" => player.hand.cards.len(),
            "discard" => player.waitroom.cards.len(),
            "energy_zone" => player.energy_zone.cards.len(),
            "live_card_zone" => player.live_card_zone.cards.len(),
            "success_live_zone" => player.success_live_card_zone.cards.len(),
            _ => 0,
        };

        if source_card_count < (count as usize) {
            eprintln!("Warning: Source '{}' has only {} cards, but trying to move {} cards. Will do as much as possible per rules 1.3.2", source, source_card_count, count);
        }

        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true,
            }
        };

        let matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        let matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
            match limit {
                Some(max_cost) => card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) <= max_cost).unwrap_or(false),
                None => true,
            }
        };

        match source.as_ref() {
            "stage" => {
                let is_self_cost = effect.self_cost.unwrap_or(false);
                let exclude_self = effect.exclude_self.unwrap_or(false);

                if is_self_cost {
                    if let Some(activating_card_id) = self.game_state.activating_card {
                        let mut found = false;
                        for i in 0..3 {
                            if player.stage.stage[i] == activating_card_id {
                                if matches_card_type(activating_card_id, card_type_filter) &&
                                   matches_group(activating_card_id, group_filter) &&
                                   matches_cost_limit(activating_card_id, cost_limit) {
                                    let card_id = player.stage.stage[i];

                                    if destination == "stage" {
                                        eprintln!("Self-cost: skipping move of activating card {} from stage to stage (no-op)", activating_card_id);
                                        found = true;
                                        break;
                                    }

                                    player.stage.stage[i] = -1;
                                    match destination.as_ref() {
                                        "discard" => { player.waitroom.add_card(card_id); }
                                        "hand" => { player.hand.add_card(card_id); }
                                        "deck_bottom" => { player.main_deck.cards.push(card_id); }
                                        "deck_top" => { player.main_deck.cards.insert(0, card_id); }
                                        "same_area" => {
                                            if let Some(activating_card_id) = self.activating_card_id {
                                                for (pos_idx, &stage_card_id) in player.stage.stage.iter().enumerate() {
                                                    if stage_card_id == activating_card_id {
                                                        player.stage.stage[pos_idx] = card_id;
                                                        break;
                                                    }
                                                }
                                            } else {
                                                player.hand.add_card(card_id);
                                            }
                                        }
                                        "live_card_zone" => {
                                            if card_type_filter.is_none() || matches_card_type(card_id, Some("live_card")) {
                                                player.live_card_zone.cards.push(card_id);
                                            } else {
                                                return Err(format!("Card {} is not a live card, cannot move to live_card_zone", card_id));
                                            }
                                        }
                                        "success_live_zone" => {
                                            if card_type_filter.is_none() || matches_card_type(card_id, Some("live_card")) {
                                                player.success_live_card_zone.cards.push(card_id);
                                            } else {
                                                return Err(format!("Card {} is not a live card, cannot move to success_live_zone", card_id));
                                            }
                                        }
                                        _ => { player.hand.add_card(card_id); }
                                    }
                                    found = true;
                                } else {
                                    return Err(format!("Activating card {} does not match cost requirements", activating_card_id));
                                }
                                break;
                            }
                        }
                        if !found {
                            return Err(format!("Activating card {} not found on stage", activating_card_id));
                        }
                    } else {
                        return Err("Self-cost required but no activating card tracked".to_string());
                    }
                } else {
                    let mut valid_cards: Vec<(usize, i16)> = Vec::new();
                    let activating_card_id = self.game_state.activating_card;

                    for i in 0..3 {
                        if player.stage.stage[i] != -1 {
                            let card_id = player.stage.stage[i];
                            if exclude_self && activating_card_id == Some(card_id) {
                                continue;
                            }
                            if matches_card_type(card_id, card_type_filter) &&
                               matches_group(card_id, group_filter) &&
                               matches_cost_limit(card_id, cost_limit) {
                                valid_cards.push((i, card_id));
                            }
                        }
                    }

                    if valid_cards.len() < (count as usize) {
                        return Err(format!("Not enough valid cards on stage: needed {}, have {} (after exclude_self filter)", count, valid_cards.len()));
                    }

                    if valid_cards.len() > (count as usize) {
                        let card_type_desc = if let Some(ct) = card_type_filter { format!("{} ", ct) } else { "".to_string() };
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "stage".to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count as usize,
                            description: format!("Select {} {}card(s) from stage to move to {} ({} available)", count, card_type_desc, destination, valid_cards.len()),
                            allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }

                    let mut cards_to_record: Vec<i16> = Vec::new();
                    for (i, card_id) in valid_cards.iter().take(count as usize) {
                        if destination == "stage" {
                            continue;
                        }
                        player.stage.stage[*i] = -1;
                        match destination.as_ref() {
                            "discard" => { player.waitroom.add_card(*card_id); }
                            "hand" => { player.hand.add_card(*card_id); }
                            "deck_bottom" => { player.main_deck.cards.push(*card_id); }
                            "deck_top" => { player.main_deck.cards.insert(0, *card_id); }
                            "live_card_zone" => {
                                if card_type_filter.is_none() || matches_card_type(*card_id, Some("live_card")) {
                                    player.live_card_zone.cards.push(*card_id);
                                } else {
                                    player.hand.add_card(*card_id);
                                }
                            }
                            "success_live_zone" => {
                                if card_type_filter.is_none() || matches_card_type(*card_id, Some("live_card")) {
                                    player.success_live_card_zone.cards.push(*card_id);
                                } else {
                                    player.hand.add_card(*card_id);
                                }
                            }
                            _ => { player.hand.add_card(*card_id); }
                        }
                        cards_to_record.push(*card_id);
                    }
                    for card_id in cards_to_record {
                        self.game_state.record_card_movement(card_id);
                    }
                }
            }
            "deck" | "deck_top" => {
                let mut moved = 0;
                let mut cards_drawn = 0;
                let mut cards_to_record: Vec<i16> = Vec::new();
                let max_draws = player.main_deck.cards.len() + count as usize;

                while moved < count && cards_drawn < max_draws {
                    if let Some(card) = player.main_deck.draw() {
                        cards_drawn += 1;
                        if matches_card_type(card, card_type_filter) && matches_group(card, group_filter) && matches_cost_limit(card, cost_limit) {
                            match destination.as_ref() {
                                "hand" => player.hand.add_card(card),
                                "discard" => player.waitroom.add_card(card),
                                "stage" => {
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                                    } else {
                                        player.hand.add_card(card);
                                    }
                                    cards_to_record.push(card);
                                }
                                "live_card_zone" => {
                                    if card_type_filter.is_none() || matches_card_type(card, Some("live_card")) {
                                        player.live_card_zone.cards.push(card);
                                    } else {
                                        player.main_deck.cards.push(card);
                                    }
                                }
                                "success_live_zone" => {
                                    if card_type_filter.is_none() || matches_card_type(card, Some("live_card")) {
                                        player.success_live_card_zone.cards.push(card);
                                    } else {
                                        player.main_deck.cards.push(card);
                                    }
                                }
                                "deck_top" => { player.main_deck.cards.insert(0, card); }
                                _ => { eprintln!("Move to destination '{}' not yet implemented", destination); }
                            }
                            moved += 1;
                        } else {
                            player.main_deck.cards.push(card);
                        }
                    } else { break; }
                }

                for card_id in cards_to_record {
                    self.game_state.record_card_movement(card_id);
                }
            }
            "hand" => {
                match destination.as_ref() {
                    "discard" => {
                        let card_type_desc = if let Some(ct) = card_type_filter { format!("{} ", ct) } else { "".to_string() };
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "hand".to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count as usize,
                            description: format!("Select {} {}card(s) from hand to discard", count, card_type_desc),
                            allow_skip: false,
                        });
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        return Ok(());
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.push(card);
                        }
                        for card_id in cards_to_clear { self.game_state.clear_modifiers_for_card(card_id); }
                    }
                    "deck_top" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.insert(0, card);
                        }
                        for card_id in cards_to_clear { self.game_state.clear_modifiers_for_card(card_id); }
                    }
                    "stage" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_record.push(card);
                            if player.stage.stage[1] == -1 {
                                player.stage.stage[1] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                            } else if player.stage.stage[0] == -1 {
                                player.stage.stage[0] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                            } else if player.stage.stage[2] == -1 {
                                player.stage.stage[2] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                            } else { player.hand.add_card(card); }
                        }
                        for card_id in cards_to_record { self.game_state.clear_modifiers_for_card(card_id); }
                    }
                    "live_card_zone" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, Some("live_card")) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_clear.push(card);
                            player.live_card_zone.cards.push(card);
                        }
                        for card_id in cards_to_clear { self.game_state.clear_modifiers_for_card(card_id); }
                    }
                    _ => {}
                }
            }
            "discard" => {
                match destination.as_ref() {
                    "hand" => {
                        let matching_indices: Vec<usize> = player.waitroom.cards.iter().enumerate()
                            .filter(|(_, card)| matches_card_type(**card, card_type_filter) && matches_group(**card, group_filter) && matches_cost_limit(**card, cost_limit))
                            .map(|(i, _)| i).collect();

                        if matching_indices.len() < (count as usize) {
                            return Err(format!("Not enough cards in discard: needed {}, have {}", count, matching_indices.len()));
                        }

                        if matching_indices.len() > (count as usize) {
                            let card_type_desc = if let Some(ct) = card_type_filter { format!("{} ", ct) } else { "".to_string() };
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "discard".to_string(),
                                card_type: card_type_filter.map(|s| s.to_string()),
                                count: count as usize,
                                description: format!("Select {} {}card(s) from discard to add to hand ({} available)", count, card_type_desc, matching_indices.len()),
                                allow_skip: false,
                            });
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }

                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            player.hand.add_card(card);
                        }
                        for card_id in cards_to_record { self.game_state.record_card_movement(card_id); }
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            player.main_deck.cards.push(card);
                        }
                        for card_id in &cards_to_record { self.game_state.clear_modifiers_for_card(*card_id); }
                        for card_id in &cards_to_record { self.game_state.record_card_movement(*card_id); }
                    }
                    "deck_top" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            player.main_deck.cards.insert(0, card);
                        }
                        for card_id in &cards_to_record { self.game_state.clear_modifiers_for_card(*card_id); }
                        for card_id in &cards_to_record { self.game_state.record_card_movement(*card_id); }
                    }
                    "deck" => {
                        let position_info = effect.position.as_ref();
                        let placement_order = effect.placement_order.as_deref();

                        if placement_order == Some("any_order") && count > 1 {
                            let mut moved = 0;
                            let mut indices_to_remove = Vec::new();
                            for (i, card) in player.waitroom.cards.iter().enumerate() {
                                if moved >= count { break; }
                                if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                    indices_to_remove.push(i); moved += 1;
                                }
                            }
                            let mut cards_to_place: Vec<i16> = Vec::new();
                            for i in indices_to_remove.into_iter().rev() {
                                let card = player.waitroom.cards.remove(i);
                                cards_to_place.push(card);
                            }
                            for card_id in cards_to_place.iter() { self.game_state.clear_modifiers_for_card(*card_id); }
                            let card_db = self.game_state.card_database.clone();
                            let card_names: Vec<String> = cards_to_place.iter()
                                .map(|&card_id| card_db.get_card(card_id).map(|c| c.name.clone()).unwrap_or(format!("Card {}", card_id)))
                                .collect();
                            self.pending_choice = Some(Choice::SelectTarget {
                                target: cards_to_place.iter().map(|&id| id.to_string()).collect::<Vec<_>>().join("|"),
                                description: format!("Choose order for cards to place on deck:\n{}",
                                    card_names.iter().enumerate().map(|(i, name)| format!("{}. {}", i + 1, name)).collect::<Vec<_>>().join("\n")),
                            });
                            self.looked_at_cards = cards_to_place;
                            self.execution_context = ExecutionContext::LookAndSelect {
                                step: super::types::LookAndSelectStep::Finalize { destination: "deck".to_string() },
                            };
                            return Ok(());
                        }

                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            if let Some(pos_info) = position_info {
                                let deck_len = player.main_deck.cards.len();
                                let insert_index = if let Some(pos_str) = pos_info.get_position() {
                                    if let Ok(pos_num) = pos_str.parse::<usize>() {
                                        if pos_num > deck_len { deck_len } else { pos_num.saturating_sub(1) }
                                    } else { 0 }
                                } else { 0 };
                                player.main_deck.cards.insert(insert_index, card);
                            } else { player.main_deck.cards.insert(0, card); }
                        }
                        for card_id in &cards_to_record { self.game_state.clear_modifiers_for_card(*card_id); }
                        for card_id in &cards_to_record { self.game_state.record_card_movement(*card_id); }
                    }
                    "stage" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, Some("member_card")) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            let available_areas: Vec<&str> = vec![
                                if player.stage.stage[1] == -1 { Some("center") } else { None },
                                if player.stage.stage[0] == -1 { Some("left_side") } else { None },
                                if player.stage.stage[2] == -1 { Some("right_side") } else { None },
                            ].into_iter().filter_map(|x| x).collect();

                            if available_areas.len() > 1 {
                                let areas_str = available_areas.join(", ");
                                self.pending_choice = Some(Choice::SelectPosition {
                                    position: areas_str.clone(),
                                    description: format!("Select stage area to place card (available: {})", areas_str),
                                });
                                self.looked_at_cards.push(card);
                                self.execution_context = ExecutionContext::LookAndSelect {
                                    step: super::types::LookAndSelectStep::Finalize { destination: "stage".to_string() },
                                };
                                return Ok(());
                            } else if available_areas.len() == 1 {
                                let area = available_areas[0];
                                match area {
                                    "center" => { player.stage.stage[1] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center); }
                                    "left_side" => { player.stage.stage[0] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide); }
                                    "right_side" => { player.stage.stage[2] = card; player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide); }
                                    _ => { player.hand.add_card(card); }
                                }
                            } else { player.hand.add_card(card); }
                        }
                        for card_id in cards_to_record { self.game_state.clear_modifiers_for_card(card_id); }
                    }
                    _ => {}
                }
            }
            "energy_zone" => {
                match destination.as_ref() {
                    "hand" => {
                        let matching_indices: Vec<usize> = player.energy_zone.cards.iter().enumerate()
                            .filter(|(_, card)| matches_card_type(**card, card_type_filter))
                            .map(|(i, _)| i).collect();

                        if matching_indices.len() < (count as usize) {
                            return Err(format!("Not enough cards in energy zone: needed {}, have {}", count, matching_indices.len()));
                        }

                        if matching_indices.len() > (count as usize) {
                            let card_type_desc = if let Some(ct) = card_type_filter { format!("{} ", ct) } else { "".to_string() };
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "energy_zone".to_string(),
                                card_type: card_type_filter.map(|s| s.to_string()),
                                count: count as usize,
                                description: format!("Select {} {}card(s) from energy zone to add to hand ({} available)", count, card_type_desc, matching_indices.len()),
                                allow_skip: false,
                            });
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }

                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.energy_zone.cards.iter().enumerate() {
                            if indices_to_remove.len() >= count as usize { break; }
                            if matches_card_type(*card, card_type_filter) {
                                indices_to_remove.push(i);
                            }
                        }
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.energy_zone.cards.remove(i);
                            player.hand.add_card(card);
                        }
                    }
                    "discard" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.energy_zone.cards.iter().enumerate() {
                            if moved >= count { break; }
                            if matches_card_type(*card, card_type_filter) {
                                indices_to_remove.push(i); moved += 1;
                            }
                        }
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.energy_zone.cards.remove(i);
                            player.waitroom.add_card(card);
                        }
                    }
                    _ => {}
                }
            }
            "live_card_zone" => {
                match destination.as_ref() {
                    "hand" => {
                        let matching_indices: Vec<usize> = player.live_card_zone.cards.iter().enumerate()
                            .filter(|(_, _)| true)
                            .map(|(i, _)| i).collect();

                        if matching_indices.len() < (count as usize) {
                            return Err(format!("Not enough cards in live card zone: needed {}, have {}", count, matching_indices.len()));
                        }

                        if matching_indices.len() > (count as usize) {
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "live_card_zone".to_string(),
                                card_type: None,
                                count: count as usize,
                                description: format!("Select {} card(s) from live card zone to add to hand ({} available)", count, matching_indices.len()),
                                allow_skip: false,
                            });
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }

                        let mut indices_to_remove = Vec::new();
                        for i in 0..count as usize {
                            if i < player.live_card_zone.cards.len() {
                                indices_to_remove.push(i);
                            }
                        }
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.hand.add_card(card);
                        }
                    }
                    "success_live_zone" => {
                        let mut indices_to_remove = Vec::new();
                        for i in 0..count as usize {
                            if i < player.live_card_zone.cards.len() {
                                indices_to_remove.push(i);
                            }
                        }
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.success_live_card_zone.cards.push(card);
                        }
                    }
                    "discard" => {
                        let mut indices_to_remove = Vec::new();
                        for i in 0..count as usize {
                            if i < player.live_card_zone.cards.len() {
                                indices_to_remove.push(i);
                            }
                        }
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.waitroom.add_card(card);
                        }
                    }
                    _ => {}
                }
            }
            "success_live_zone" => {
                match destination.as_ref() {
                    "hand" => {
                        let matching_indices: Vec<usize> = player.success_live_card_zone.cards.iter().enumerate()
                            .map(|(i, _)| i).collect();

                        if matching_indices.len() < (count as usize) {
                            return Err(format!("Not enough cards in success live zone: needed {}, have {}", count, matching_indices.len()));
                        }

                        if matching_indices.len() > (count as usize) {
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "success_live_zone".to_string(),
                                card_type: None,
                                count: count as usize,
                                description: format!("Select {} card(s) from success live zone to hand ({} available)", count, matching_indices.len()),
                                allow_skip: false,
                            });
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            return Ok(());
                        }

                        let mut indices_to_remove = Vec::new();
                        for i in 0..count as usize {
                            if i < player.success_live_card_zone.cards.len() {
                                indices_to_remove.push(i);
                            }
                        }
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            player.hand.add_card(card);
                        }
                    }
                    "deck_top" => {
                        let mut indices_to_remove = Vec::new();
                        for i in 0..count as usize {
                            if i < player.success_live_card_zone.cards.len() {
                                indices_to_remove.push(i);
                            }
                        }
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.insert(0, card);
                        }
                        for card_id in cards_to_clear { self.game_state.clear_modifiers_for_card(card_id); }
                    }
                    "deck_bottom" => {
                        let mut indices_to_remove = Vec::new();
                        for i in 0..count as usize {
                            if i < player.success_live_card_zone.cards.len() {
                                indices_to_remove.push(i);
                            }
                        }
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.push(card);
                        }
                        for card_id in cards_to_clear { self.game_state.clear_modifiers_for_card(card_id); }
                    }
                    _ => {}
                }
            }
            _ => {
                eprintln!("Unsupported move: {} -> {}", source, destination);
            }
        }

        // Publish card moved event
        let activating_card_id = self.game_state.activating_card;
        if let Some(card_id) = activating_card_id {
            let pid = self.game_state.active_player().id.clone();
            self.game_state.publish_event(crate::events::GameEvent::CardMoved {
                card_id, from: source.clone(), to: destination.clone(), player_id: pid,
            });
        }

        Ok(())
    }
}
