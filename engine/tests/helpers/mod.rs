/// Shared test helpers for gameplay integration tests.
///
/// Every test loads the real card database, then uses `TestGame` to
/// set up a board state and play through a scenario.
///
/// Filler cards (zero abilities, no ability triggers) are available in
/// `tests/data/cards.json` and can be referenced by card_no.

use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;

/// Load the real card database from `cards/cards.json` + `cards/abilities.json`.
/// This includes ALL real cards (both the tested ability cards and filler cards).
pub fn load_real_database() -> Arc<CardDatabase> {
    let cards_path = Path::new("../cards/cards.json");
    let cards = CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load real cards from ../cards/cards.json");
    let db = CardDatabase::load_or_create(cards);
    Arc::new(db)
}

/// Get a card's database ID by its card_no string.
pub fn card_id(db: &CardDatabase, card_no: &str) -> i16 {
    db.get_card_id(card_no)
        .unwrap_or_else(|| panic!("Card {card_no} not found in database"))
}

// ====================================================================
// TestGame — scenario-based game state wrapper
// ====================================================================
// Usage:
//   let db = load_real_database();
//   let mut game = TestGame::new(db);
//   let ruby = game.id("PL!S-bp2-009-R");
//   let filler = game.id("PL!-sd1-010-SD");
//   game.add_to_hand(ruby);
//   game.add_to_discard(filler);
//   game.give_energy(3);
//   game.play_to_stage(ruby, MemberArea::Center);
//   game.activate_ability(ruby);
//   game.select_indices(&[0]);
//   assert!(game.player().hand.cards.contains(&filler));
// ====================================================================

pub struct TestGame {
    pub db: Arc<CardDatabase>,
    pub state: GameState,
}

#[allow(dead_code)]
impl TestGame {
    /// Create a fresh game in Main phase, turn 1 (skips RPS/mulligan/setup).
    pub fn new(db: Arc<CardDatabase>) -> Self {
        let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
        let p2 = Player::new("p2".into(), "Player 2".into(), false);
        p1.is_first_attacker = true;

        let mut state = GameState::new(p1, p2, db.clone());
        state.current_phase = Phase::Main;
        state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        state.turn_number = 1;

        TestGame { db, state }
    }

    /// Look up a card's numeric ID by card_no in the database.
    pub fn id(&self, card_no: &str) -> i16 {
        card_id(&self.db, card_no)
    }

    /// Shortcut for `state.player1` (the active player in our tests).
    pub fn player(&mut self) -> &mut Player {
        &mut self.state.player1
    }

    // ---- Zone setup ----

    /// Put a card in player1's hand.
    pub fn add_to_hand(&mut self, id: i16) {
        self.state.player1.hand.cards.push(id);
    }

    /// Put a card in player1's waitroom (discard).
    pub fn add_to_discard(&mut self, id: i16) {
        self.state.player1.waitroom.cards.push(id);
    }

    /// Put a card on player1's stage at the given area.
    pub fn add_to_stage(&mut self, area: MemberArea, id: i16) {
        self.state.player1.stage.set_area(area, id);
    }

    /// Give player1 active energy (uses real card LL-E-001-SD).
    pub fn give_energy(&mut self, count: usize) {
        let energy_card = self.id("LL-E-001-SD");
        for _ in 0..count {
            self.state.player1.energy_zone.cards.push(energy_card);
        }
        self.state.player1.energy_zone.active_energy_count += count;
    }

    // ---- Actions ----

    /// Play a member card from hand onto the stage.
    pub fn play_to_stage(&mut self, card_id: i16, area: MemberArea) {
        TurnEngine::execute_main_phase_action(
            &mut self.state,
            &ActionType::PlayMemberToStage,
            Some(card_id),
            None,
            Some(area),
            Some(false),
        )
        .expect("play_to_stage failed");
    }

    /// Activate the first 起動 (activation) ability on a stage card.
    pub fn activate_ability(&mut self, stage_card_id: i16) {
        TurnEngine::execute_main_phase_action(
            &mut self.state,
            &ActionType::UseAbility,
            Some(stage_card_id),
            None,
            None,
            None,
        )
        .expect("activate_ability failed");
    }

    /// Set a live card from hand during LiveCardSet phase.
    pub fn set_live_card(&mut self, card_id: i16) {
        TurnEngine::execute_main_phase_action(
            &mut self.state,
            &ActionType::SetLiveCard,
            Some(card_id),
            None,
            None,
            None,
        )
        .expect("set_live_card failed");
    }

    /// Check if the ability queue is waiting for a player choice.
    pub fn has_pending_choice(&self) -> bool {
        self.state.pending_choice.is_some()
            || self.state.ability_queue.is_waiting_for_choice().is_some()
    }

    /// Select cards by waitroom/hand indices (for SelectCard choices).
    pub fn select_indices(&mut self, indices: &[usize]) {
        TurnEngine::resume_with_choice(&mut self.state, None, Some(indices.to_vec()))
            .expect("select_indices failed");
    }

    /// Select a choice option by index (for SelectTarget choices like answers, alternatives).
    pub fn select_option(&mut self, option_index: i16) {
        TurnEngine::resume_with_choice(&mut self.state, Some(option_index), None)
            .expect("select_option failed");
    }

    /// Advance to the next phase (Pass action).
    pub fn pass(&mut self) {
        TurnEngine::execute_main_phase_action(
            &mut self.state,
            &ActionType::Pass,
            None, None, None, None,
        )
        .expect("pass failed");
    }

    // ---- Debugging ----

    /// Resolve a card ID to its name from the database.
    pub fn name(&self, id: i16) -> String {
        self.db.get_card(id).map(|c| format!("{} ({})", c.name, c.card_no)).unwrap_or_else(|| format!("#{}", id))
    }

    /// Print card IDs in player1's hand.
    pub fn dbg_hand(&self) {
        let cards: Vec<String> = self.state.player1.hand.cards.iter().map(|&id| self.name(id)).collect();
        eprintln!("[HAND] {:?}", cards);
    }

    /// Print card IDs in player1's waitroom.
    pub fn dbg_discard(&self) {
        let cards: Vec<String> = self.state.player1.waitroom.cards.iter().map(|&id| self.name(id)).collect();
        eprintln!("[DISCARD] {:?}", cards);
    }

    /// Print cards on player1's stage.
    pub fn dbg_stage(&self) {
        let cards: Vec<String> = self.state.player1.stage.stage.iter().map(|&id| {
            if id == -1 { "empty".into() } else { self.name(id) }
        }).collect();
        eprintln!("[STAGE] {:?}", cards);
    }

    /// Print the current pending choice details.
    pub fn dbg_choice(&self) {
        if let Some(choice) = self.state.ability_queue.is_waiting_for_choice() {
            eprintln!("[CHOICE] {:?}", choice);
        } else if let Some(ref pc) = self.state.pending_choice {
            eprintln!("[CHOICE] (json) {:?}", pc);
        } else {
            eprintln!("[CHOICE] none");
        }
    }

    /// Print all zones and pending state at once.
    pub fn dbg_all(&self) {
        self.dbg_hand();
        self.dbg_discard();
        self.dbg_stage();
        self.dbg_choice();
    }
}
