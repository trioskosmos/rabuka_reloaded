use crate::core::game_state::GameState;
use crate::core::game_state::PermanentLoopProtocol;
use crate::core::types::GameResult;
use crate::core::constants::CountCast;

impl GameState {
    pub fn set_opponent_live_success(&mut self, no_excess_heart: bool) {
        self.opponent_live_success_this_turn = true;
        self.opponent_live_no_excess_heart_this_turn = no_excess_heart;
    }

    pub fn reset_change_flags(&mut self) {
        self.position_change_occurred_this_turn = false;
        self.formation_change_occurred_this_turn = false;
        self.opponent_live_success_this_turn = false;
        self.opponent_live_no_excess_heart_this_turn = false;
        self.p1_live_success_this_turn = false;
        self.p1_live_success_no_excess = false;
        self.p2_live_success_this_turn = false;
        self.p2_live_success_no_excess = false;
        self.live_success_triggered_this_turn = false;
        self.live_success_p2_fired = false;
        self.live_success_p1_fired = false;
        self.live_success_p1_extra = 0;
        self.live_success_p2_extra = 0;
        self.last_state_change_wait_to_active_count = 0;
        self.clear_state_change_window("Active-phase entry / turn boundary");
        // NOTE: turn_state_changes is NOT cleared here -- this runs on every
        // Active-phase entry (each player's normal phase), but 「このターン、E        // spans the whole round (both players' main phases + live). It is
        // cleared at the real turn boundary in advance_phase (victory
        // determination, turn_number increment).
        self.self_no_excess_heart_this_turn = false;
    }

    pub fn check_permanent_loop(&mut self) -> bool {
        let state_hash = self.generate_state_hash();
        if self.game_state_history.contains(&state_hash) {
            self.loop_detected = true;
            return true;
        }
        self.game_state_history.push(state_hash);
        false
    }

    pub fn record_action_boundary(&mut self, action: crate::game_setup::ActionType) {
        // This is the only caller-visible cost of the Rule 12-1 loop protocol
        // (full-board state hash + O(n) history scan) and it is invisible in
        // profile_target, which calls execute_main_phase_action directly and
        // never reaches here. Time it or it stays invisible.
        #[cfg(not(feature = "no_std"))]
        let _timer = crate::timer::Timer::start("record_action_boundary");
        if self.loop_last_action != Some(action) {
            self.game_state_history.clear();
            self.pending_loop_protocol = None;
            self.loop_last_action = Some(action);
        }
        if self.pending_loop_protocol.is_some() {
            return;
        }
        let hash = self.generate_state_hash();
        self.game_state_history.push(hash);
        let repetition_count = self
            .game_state_history
            .iter()
            .filter(|&&seen| seen == hash)
            .count()
            .u8_count();
        if repetition_count < 3 {
            return;
        }
        self.pending_loop_protocol = Some(PermanentLoopProtocol {
            choice: crate::ability::types::Choice::SelectTarget {
                target: "rule_12_1".to_string(),
                description: "Stop the repeated action and draw the game?".to_string(),
                description_en: Some("Stop the repeated action and draw the game?".to_string()),
                description_ja: Some("繰り返しを停止して引き分けにしますか？".to_string()),
                allow_skip: false,
                options: Some(vec!["Stop".to_string(), "Continue".to_string()]),
            },
        });
        self.loop_detected = true;
    }

    pub fn resolve_loop_protocol(&mut self, continue_loop: bool) {
        if self.pending_loop_protocol.take().is_none() {
            return;
        }
        if !continue_loop {
            self.game_result = GameResult::Draw;
            self.game_ended = true;
            return;
        }
        self.game_state_history.clear();
        self.loop_detected = false;
    }

    pub fn generate_state_hash(&self) -> u64 {
        use core::hash::{Hash, Hasher};
        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn write(&mut self, bytes: &[u8]) {
                for &b in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
                }
            }
            fn finish(&self) -> u64 {
                self.0
            }
        }
        let mut hasher = SimpleHasher(0);
        self.turn_number.hash(&mut hasher);
        self.current_phase.hash(&mut hasher);
        self.current_turn_phase.hash(&mut hasher);
        self.player1.hand.cards.hash(&mut hasher);
        self.player1.main_deck.cards.hash(&mut hasher);
        self.player1.energy_deck.cards.hash(&mut hasher);
        self.player1.energy_zone.cards.hash(&mut hasher);
        self.player1.waitroom.cards.hash(&mut hasher);
        self.player1.live_card_zone.cards.hash(&mut hasher);
        self.player1.success_live_card_zone.cards.hash(&mut hasher);
        self.player1.stage.stage.hash(&mut hasher);
        for under_cards in &self.player1.stage.under_cards {
            under_cards.hash(&mut hasher);
        }
        self.player2.hand.cards.hash(&mut hasher);
        self.player2.main_deck.cards.hash(&mut hasher);
        self.player2.energy_deck.cards.hash(&mut hasher);
        self.player2.energy_zone.cards.hash(&mut hasher);
        self.player2.waitroom.cards.hash(&mut hasher);
        self.player2.live_card_zone.cards.hash(&mut hasher);
        self.player2.success_live_card_zone.cards.hash(&mut hasher);
        self.player2.stage.stage.hash(&mut hasher);
        for under_cards in &self.player2.stage.under_cards {
            under_cards.hash(&mut hasher);
        }
        self.mods.orientation_modifiers.len().hash(&mut hasher);
        self.prohibition_effects.len().hash(&mut hasher);
        self.temporary_effects.len().hash(&mut hasher);
        self.mulligan_selected_indices.hash(&mut hasher);
        self.live_card_selected_indices.hash(&mut hasher);
        self.rps_winner.hash(&mut hasher);
        hasher.finish()
    }

    pub fn reset_loop_detection(&mut self) {
        self.game_state_history.clear();
        self.loop_detected = false;
        self.pending_loop_protocol = None;
        self.loop_last_action = None;
    }

    pub fn is_loop_detected(&self) -> bool {
        self.loop_detected
    }
}
