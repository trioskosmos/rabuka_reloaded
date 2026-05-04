impl GameState {

    pub fn add_blade_modifier(&mut self, card_id: i16, delta: i32) {
        *self.blade_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn remove_blade_modifier(&mut self, card_id: i16, delta: i32) {
        let val = self.blade_modifiers.entry(card_id).or_insert(0);
        *val -= delta;
        if *val == 0 {
            self.blade_modifiers.remove(card_id);
        }
    }

    pub fn get_blade_modifier(&self, card_id: i16) -> i32 {
        self.blade_modifiers.get(card_id).copied().unwrap_or(0)
    }

    pub fn set_blade_type_modifier(&mut self, card_id: i16, blade_color: BladeColor) {
        self.blade_type_modifiers.set(card_id, blade_color);
    }

    pub fn get_blade_type_modifier(&self, card_id: i16) -> Option<BladeColor> {
        self.blade_type_modifiers.get(card_id).copied()
    }

    pub fn clear_blade_type_modifier(&mut self, card_id: i16) {
        self.blade_type_modifiers.remove(card_id);
    }

    /// Recalculate all 常時 (constant) ability blade bonuses.
    /// Evaluates each constant ability's condition against the current game state
    /// and applies/removes blade modifiers accordingly.
    pub fn recalculate_constant_blade_modifiers(&mut self) {
        // Collect blade-granting constant abilities from all stage cards
        let mut blade_abilities: Vec<(i16, crate::card::AbilityEffect)> = Vec::new();
        for &cid in self.player1.stage.stage.iter().chain(self.player2.stage.stage.iter()) {
            if cid == -1 { continue; }
            let card = match self.card_database.get_card(cid) { Some(c) => c, None => continue };
            for ability in &card.abilities {
                if ability.triggers.as_ref().map_or(false, |t| t.contains(crate::triggers::CONSTANT)) {
                    if let Some(ref effect) = ability.effect {
                        if effect.action == "gain_resource" && matches!(effect.resource.as_deref(), Some("blade") | Some("ブレード")) {
                            blade_abilities.push((cid, effect.clone()));
                        }
                    }
                }
            }
        }

        // Evaluate conditions and sum expected bonuses
        let mut expected: HashMap<i16, i32> = HashMap::new();
        {
            let resolver = crate::ability::resolver::AbilityResolver::new(self);
            for &(cid, ref effect) in &blade_abilities {
                let cond_met = effect.condition.as_ref().map_or(true, |c| resolver.evaluate_condition(c));
                if cond_met {
                    let count = effect.resource_icon_count.unwrap_or(effect.count.unwrap_or(1));
                    *expected.entry(cid).or_insert(0) += count as i32;
                }
            }
        }

        // Remove old bonuses (clone before mutating self)
        let old_bonuses = std::mem::take(&mut self.constant_blade_bonuses);
        for (cid, old) in &old_bonuses { self.remove_blade_modifier(*cid, *old); }
        // Apply new bonuses
        for (&cid, &new_val) in &expected { self.add_blade_modifier(cid, new_val); }
        self.constant_blade_bonuses = expected;
    }

    pub fn add_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, delta: i32) {
        let colors = self.heart_modifiers.entry(card_id).or_insert_with(std::collections::HashMap::new);
        *colors.entry(color).or_insert(0) += delta;
    }

    pub fn remove_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, delta: i32) {
        if let Some(colors) = self.heart_modifiers.get_mut(&card_id) {
            if let Some(modifier) = colors.get_mut(&color) {
                *modifier -= delta;
                if *modifier == 0 {
                    colors.remove(&color);
                }
            }
            if colors.is_empty() {
                self.heart_modifiers.remove(&card_id);
            }
        }
    }

    pub fn get_heart_modifier(&self, card_id: i16, color: crate::card::HeartColor) -> i32 {
        self.heart_modifiers.get(&card_id)
            .and_then(|colors| colors.get(&color))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_heart_override(&mut self, card_id: i16, color: crate::card::HeartColor, count: u32, duration: &str) {
        self.heart_override.insert(card_id, (color, count));
        let mut data = serde_json::Map::new();
        data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
        data.insert("color".to_string(), serde_json::Value::String(format!("{:?}", color)));
        data.insert("count".to_string(), serde_json::Value::Number(count.into()));
        self.temporary_effects.push(TemporaryEffect {
            effect_type: "heart_override".to_string(),
            duration: match duration { "live_end" => Duration::LiveEnd, "this_turn" => Duration::ThisTurn, _ => Duration::ThisLive },
            created_turn: self.turn_number,
            created_phase: self.current_phase.clone(),
            target_player_id: String::new(),
            description: format!("Heart override: card {} = {:?} x{}", card_id, color, count),
            creation_order: 0,
            effect_data: Some(serde_json::Value::Object(data)),
        });
    }

    pub fn add_score_modifier(&mut self, card_id: i16, delta: i32) {
        *self.score_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn get_score_modifier(&self, card_id: i16) -> i32 {
        self.score_modifiers.get(card_id).copied().unwrap_or(0)
    }

    pub fn set_score_modifier(&mut self, card_id: i16, value: i32) {
        self.score_modifiers.set(card_id, value);
    }

    pub fn add_need_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, delta: i32) {
        let colors = self.need_heart_modifiers.entry(card_id).or_insert_with(std::collections::HashMap::new);
        *colors.entry(color).or_insert(0) += delta;
    }

    pub fn get_need_heart_modifier(&self, card_id: i16, color: crate::card::HeartColor) -> i32 {
        self.need_heart_modifiers.get(&card_id)
            .and_then(|colors| colors.get(&color))
            .copied()
            .unwrap_or(0)
    }

    pub fn record_area_placement(&mut self, player_id: &str, area: &str) {
        let key = format!("{}:{}", player_id, area);
        self.areas_placed_this_turn.insert(key);
    }

    pub fn has_area_been_placed_this_turn(&self, player_id: &str, area: &str) -> bool {
        let key = format!("{}:{}", player_id, area);
        self.areas_placed_this_turn.contains(&key)
    }

    pub fn clear_area_placement_tracking(&mut self) {
        self.areas_placed_this_turn.clear();
    }

    pub fn record_card_appearance(&mut self, card_id: i16) {
        self.cards_appeared_this_turn.insert(card_id);
    }

    pub fn has_card_appeared_this_turn(&self, card_id: i16) -> bool {
        self.cards_appeared_this_turn.contains(&card_id)
    }

    pub fn clear_card_appearance_tracking(&mut self) {
        self.cards_appeared_this_turn.clear();
    }

    pub fn set_turn_order_changed(&mut self, changed: bool) {
        self.turn_order_changed = changed;
    }

    pub fn has_turn_order_changed(&self) -> bool {
        self.turn_order_changed
    }

    pub fn record_auto_ability_trigger(&mut self, card_id: &str) {
        *self.auto_ability_trigger_counts.entry(card_id.to_string()).or_insert(0) += 1;
    }

    pub fn get_auto_ability_trigger_count(&self, card_id: &str) -> u32 {
        *self.auto_ability_trigger_counts.get(card_id).unwrap_or(&0)
    }

    pub fn clear_auto_ability_trigger_tracking(&mut self) {
        self.auto_ability_trigger_counts.clear();
    }

    pub fn record_turn_limit_usage(&mut self, player_id: &str, card_instance_id: u32) {
        let key = format!("{}:{}", player_id, card_instance_id);
        *self.turn_limit_usage.entry(key).or_insert(0) += 1;
    }

    pub fn get_turn_limit_usage(&self, player_id: &str, card_instance_id: u32) -> u32 {
        let key = format!("{}:{}", player_id, card_instance_id);
        *self.turn_limit_usage.get(&key).unwrap_or(&0)
    }

    pub fn clear_turn_limit_tracking(&mut self) {
        self.turn_limit_usage.clear();
    }

    pub fn assign_card_instance_id(&mut self, card_id: i16) -> u32 {
        self.card_instance_counter += 1;
        let instance_id = self.card_instance_counter;
        self.card_instance_mapping.insert(card_id, instance_id);
        instance_id
    }

    pub fn get_card_instance_id(&self, card_id: i16) -> Option<u32> {
        self.card_instance_mapping.get(&card_id).copied()
    }

    pub fn remove_card_instance(&mut self, card_id: i16) {
        self.card_instance_mapping.remove(&card_id);
    }

    pub fn clear_card_instance_tracking(&mut self) {
        self.card_instance_mapping.clear();
        self.card_instance_counter = 0;
    }

    pub fn record_baton_touch(&mut self) {
        self.baton_touch_count += 1;
    }

    pub fn get_baton_touch_count(&self) -> u32 {
        self.baton_touch_count
    }

    pub fn clear_baton_touch_tracking(&mut self) {
        self.baton_touch_count = 0;
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
    }

    pub fn record_card_movement(&mut self, card_id: i16) {
        self.cards_moved_this_turn.insert(card_id);
    }

    pub fn has_card_moved_this_turn(&self, card_id: i16) -> bool {
        self.cards_moved_this_turn.contains(&card_id)
    }

    pub fn clear_card_movement_tracking(&mut self) {
        self.cards_moved_this_turn.clear();
    }

    pub fn set_heart_color_decision_phase(&mut self, phase: &str) {
        self.heart_color_decision_phase = phase.to_string();
    }

    pub fn get_heart_color_decision_phase(&self) -> &str {
        &self.heart_color_decision_phase
    }

    pub fn is_in_required_hearts_check_phase(&self) -> bool {
        self.heart_color_decision_phase == "required_hearts_check"
    }

    pub fn is_in_live_start_phase(&self) -> bool {
        self.heart_color_decision_phase == "live_start"
    }

    pub fn set_deck_refresh_pending(&mut self, pending: bool) {
        self.deck_refresh_pending = pending;
    }

    pub fn is_deck_refresh_pending(&self) -> bool {
        self.deck_refresh_pending
    }

    pub fn perform_deck_refresh(&mut self, player_id: &str) {
        let player = if player_id == "player1" {
            &mut self.player1
        } else {
            &mut self.player2
        };

        let waitroom_cards: Vec<i16> = player.waitroom.cards.iter().copied().collect();
        player.waitroom.cards.clear();
        for card_id in waitroom_cards {
            player.main_deck.cards.push(card_id);
        }

        player.main_deck.shuffle();
        self.deck_refresh_pending = false;
    }



    pub fn set_live_being_performed(&mut self, performed: bool) {
        self.live_being_performed = performed;
    }

    pub fn is_live_being_performed(&self) -> bool {
        self.live_being_performed
    }

    pub fn set_game_ended(&mut self, ended: bool) {
        self.game_ended = ended;
    }

    pub fn is_game_ended(&self) -> bool {
        self.game_ended
    }

    pub fn set_draw_state(&mut self, draw: bool) {
        self.draw_state = draw;
    }

    pub fn is_draw_state(&self) -> bool {
        self.draw_state
    }

    pub fn check_success_zone_draw_condition(&self, player_id: &str) -> bool {
        let player = if player_id == self.player1.id {
            &self.player1
        } else if player_id == self.player2.id {
            &self.player2
        } else {
            return false;
        };

        let success_count = player.success_live_card_zone.cards.len();
        success_count >= 3
    }

    pub fn add_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.insert(card_id);
    }

    pub fn remove_revealed_card(&mut self, card_id: i16) {
        self.revealed_cards.remove(&card_id);
    }

    pub fn is_card_revealed(&self, card_id: i16) -> bool {
        self.revealed_cards.contains(&card_id)
    }

    pub fn clear_revealed_cards(&mut self) {
        self.revealed_cards.clear();
    }

    pub fn add_gained_ability(&mut self, card_id: i16, ability_type: String) {
        self.gained_abilities.entry(card_id).or_insert_with(Vec::new).push(ability_type);
    }

    pub fn remove_gained_abilities(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }

    pub fn has_gained_ability(&self, card_id: i16, ability_type: &str) -> bool {
        if let Some(abilities) = self.gained_abilities.get(&card_id) {
            abilities.iter().any(|a| a == ability_type)
        } else {
            false
        }
    }

    pub fn clear_gained_abilities_for_card(&mut self, card_id: i16) {
        self.gained_abilities.remove(&card_id);
    }

    pub fn set_need_heart_modifier(&mut self, card_id: i16, color: crate::card::HeartColor, value: i32) {
        self.need_heart_modifiers.entry(card_id).or_default().insert(color, value);
    }

    pub fn add_orientation_modifier(&mut self, card_id: i16, orientation: &str) {
        self.orientation_modifiers.set(card_id, orientation.to_string());
    }

    pub fn add_cost_modifier(&mut self, card_id: i16, delta: i32) {
        *self.cost_modifiers.entry(card_id).or_insert(0) += delta;
    }

    pub fn set_cost_modifier(&mut self, card_id: i16, value: i32) {
        self.cost_modifiers.set(card_id, value);
    }

    pub fn get_cost_modifier(&self, card_id: i16) -> i32 {
        self.cost_modifiers.get(card_id).copied().unwrap_or(0)
    }

    pub fn get_orientation_modifier(&self, card_id: i16) -> Option<&String> {
        self.orientation_modifiers.get(card_id)
    }

    pub fn clear_modifiers_for_card(&mut self, card_id: i16) {
        self.blade_modifiers.remove(card_id);
        self.heart_modifiers.remove(&card_id);
        self.heart_override.remove(&card_id);
        self.score_modifiers.remove(card_id);
        self.need_heart_modifiers.remove(&card_id);
        self.orientation_modifiers.remove(card_id);
        self.cost_modifiers.remove(card_id);
    }
}