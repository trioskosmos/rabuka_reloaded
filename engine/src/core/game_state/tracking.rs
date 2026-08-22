use super::GameState;

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
        self.baton_touch_count_p1 = 0;
        self.baton_touch_count_p2 = 0;
        self.baton_touch_arriving_card_ids.clear();
        self.baton_touch_zero_cost = false;
        self.baton_touch_replaced_member_cost = None;
        self.baton_touch_replaced_member_id = None;
        self.baton_touch_arriving_card_id = None;
        self.clear_area_placement_tracking();
    }

    /// Register a modify_yell_count effect: (player_slot 1|2, delta).
    /// Modifiers are data; the required count is derived on read.
    pub fn add_yell_count_modifier(&mut self, player_slot: u8, delta: i32) {
        self.yell_count_modifiers.push((player_slot, delta));
    }

    /// Derived cheer checks required for `player_id`:
    /// max(0, base + Σ deltas registered for that player).
    pub fn effective_cheer_checks_required(&self, player_id: &str, base: u8) -> u8 {
        let slot = if player_id == self.player1.id || player_id == "p1" {
            1u8
        } else {
            2u8
        };
        let base = self.cheer_check_base.unwrap_or(base);
        let sum: i32 = self
            .yell_count_modifiers
            .iter()
            .filter(|m| m.0 == slot)
            .map(|m| m.1)
            .sum();
        (base as i32 + sum).max(0) as u8
    }

    pub fn perform_cheer_check(&mut self, player_id: &str, blade_count: u8) -> Result<(), String> {
        // The required count is always DERIVED: base (the live's blade count,
        // fixed at the first check) plus every modify_yell_count modifier for
        // this player, whenever it was applied. There is no initialization
        // order dependency: a LiveStart modifier registered before the base
        // exists is simply included in the sum.
        if self.cheer_check_base.is_none() {
            self.cheer_check_base = Some(blade_count);
        }
        self.cheer_checks_required = self.effective_cheer_checks_required(player_id, blade_count);

        let player = if player_id == self.player1.id {
            &mut self.player1
        } else {
            &mut self.player2
        };

        // Q104 / Rule 10.2.1: refresh from waitroom when deck runs out mid-draw.
        let from_bottom = player.yell_from_bottom;
        for _ in 0..blade_count {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            // G8: 恋になりたいAQUARIUM makes the yell reveal from the deck bottom.
            let card_id = if from_bottom {
                player.main_deck.draw_bottom()
            } else {
                player.main_deck.draw()
            };
            if let Some(card_id) = card_id {
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

    pub fn is_action_prohibited(&self, action: &str) -> bool {
        self.prohibition_effects.iter().any(|e| e.contains(action))
    }

}
