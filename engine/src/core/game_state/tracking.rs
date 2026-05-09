impl GameState {

    pub fn can_play_turn1_ability(&self, ability_id: &str) -> bool {
        !self.turn1_abilities_played.contains(ability_id)
    }

    pub fn can_play_turn2_ability(&self, ability_id: &str) -> bool {
        let count = self.turn2_abilities_played.get(ability_id).unwrap_or(&0);
        *count < 2
    }

    pub fn record_turn1_ability(&mut self, ability_id: String) {
        self.turn1_abilities_played.insert(ability_id);
    }

    pub fn record_turn2_ability(&mut self, ability_id: String) {
        *self.turn2_abilities_played.entry(ability_id).or_insert(0) += 1;
    }

    pub fn can_activate_area_ability(&self, player_id: &str, card_no: &str, area: crate::zones::MemberArea) -> bool {
        let player = if player_id == self.player1.id { &self.player1 } else { &self.player2 };
        if let Some(card_in_zone) = player.stage.get_area(area) {
            if let Some(card) = self.card_database.get_card(card_in_zone) {
                card.card_no == card_no
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn can_activate_center_ability(&self, player_id: &str, card_no: &str) -> bool {
        self.can_activate_area_ability(player_id, card_no, crate::zones::MemberArea::Center)
    }

    pub fn can_activate_left_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        self.can_activate_area_ability(player_id, card_no, crate::zones::MemberArea::LeftSide)
    }

    pub fn can_activate_right_side_ability(&self, player_id: &str, card_no: &str) -> bool {
        self.can_activate_area_ability(player_id, card_no, crate::zones::MemberArea::RightSide)
    }

    pub fn reset_keyword_tracking(&mut self) {
        self.turn1_abilities_played.clear();
        self.turn2_abilities_played.clear();
        self.player1_cheer_blade_heart_count = 0;
        self.player2_cheer_blade_heart_count = 0;
        self.player1.debut_count_this_turn = 0;
        self.player2.debut_count_this_turn = 0;
        self.clear_card_appearance_tracking();
        self.clear_auto_ability_trigger_tracking();
        self.reset_change_flags();
        self.cheer_check_completed = false;
        self.reset_loop_detection();
        self.player1.areas_locked_this_turn.clear();
        self.player2.areas_locked_this_turn.clear();
        self.baton_touch_count = 0;
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
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

        for _ in 0..blade_count {
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
            return Err(format!("Cannot check required hearts: {} of {} cheer checks completed",
                self.cheer_checks_done, self.cheer_checks_required));
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
        self.turn_limited_abilities_used.insert(card_id);
    }

    pub fn has_turn_limited_ability_been_used(&self, card_id: &str) -> bool {
        self.turn_limited_abilities_used.contains(card_id)
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