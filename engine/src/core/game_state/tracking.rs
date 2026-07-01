impl GameState {
    pub fn reset_keyword_tracking(&mut self) {
        self.turn1_abilities_played.clear();
        self.turn2_abilities_played.clear();
        self.player1_cheer_blade_heart_count = 0;
        self.player2_cheer_blade_heart_count = 0;
        self.player1.debut_count_this_turn = 0;
        self.player2.debut_count_this_turn = 0;
        self.player1.deck_refreshed_this_turn = false;
        self.player2.deck_refreshed_this_turn = false;
        self.player1.last_resolution_cards.clear();
        self.player2.last_resolution_cards.clear();
        self.clear_card_appearance_tracking();
        self.clear_auto_ability_trigger_tracking();
        self.reset_change_flags();
        self.cheer_check_completed = false;
        self.reset_loop_detection();
        self.player1.deployed_this_turn.clear();
        self.player2.deployed_this_turn.clear();
        self.baton_touch_count.clear();
        self.baton_touch_arriving_card_ids.clear();
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
        self.baton_touch_replaced_member_id = None;
        self.baton_touch_arriving_card_id = None;
        self.clear_area_placement_tracking();
    }

    pub fn perform_cheer_check(&mut self, player_id: &str, blade_count: u32) -> Result<(), String> {
        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        if self.cheer_checks_required == 0 {
            self.cheer_checks_required = blade_count;
        }

        // Q104 / Rule 10.2.1: refresh from waitroom when deck runs out mid-draw.
        for _ in 0..blade_count {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            if let Some(card_id) = player.main_deck.draw() {
                self.resolution_zone.cards.push(card_id);
                self.cheer_checks_done += 1;
            }
        }

        if self.cheer_checks_done >= self.cheer_checks_required {
            self.cheer_check_completed = true;
        }
        Ok(())
    }

    pub fn check_required_hearts(&self) -> Result<bool, String> {
        if self.cheer_checks_done < self.cheer_checks_required {
            return Err(format!(
                "Cannot check required hearts: {} of {} cheer checks completed",
                self.cheer_checks_done, self.cheer_checks_required
            ));
        }
        Ok(true)
    }

    pub fn add_prohibition_effect(&mut self, effect: String) {
        self.prohibition_effects.push(effect);
    }

    pub fn is_action_prohibited(&self, action: &str) -> bool {
        self.prohibition_effects.iter().any(|e| e.contains(action))
    }

    pub fn record_turn_limited_ability_use(&mut self, card_id: String) {
        *self.turn_limited_abilities_used.entry(card_id).or_insert(0) += 1;
    }

    pub fn has_turn_limited_ability_been_used(&self, card_id: &str) -> bool {
        self.turn_limited_abilities_used
            .get(card_id)
            .copied()
            .unwrap_or(0)
            > 0
    }

    pub fn move_resolution_zone_to_waitroom(&mut self, player_id: &str) {
        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        for card_id in self.resolution_zone.cards.drain(..) {
            player.waitroom.cards.push(card_id);
        }
    }
}
