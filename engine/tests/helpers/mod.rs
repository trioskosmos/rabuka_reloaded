/// Shared test helpers for gameplay integration tests.
///
/// Every test loads the real card database, then uses `TestGame` to
/// set up a board state and play through a scenario.
///
/// Filler cards (zero abilities, no ability triggers) are available in
/// `tests/data/cards.json` and can be referenced by card_no.
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;

use rabuka_engine::ability::types::Choice;
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
/// The database is loaded once per process and cached via `OnceLock`.
pub fn load_real_database() -> Arc<CardDatabase> {
    static DB: OnceLock<Arc<CardDatabase>> = OnceLock::new();
    DB.get_or_init(|| {
        let cards_path = Path::new("../cards/cards.json");
        let cards = CardLoader::load_cards_from_file(cards_path)
            .expect("Failed to load real cards from ../cards/cards.json");
        Arc::new(CardDatabase::load_or_create(cards))
    })
    .clone()
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
    copy_pool: RefCell<HashMap<i16, Vec<i16>>>,
}

#[allow(dead_code)]
impl TestGame {
    /// Create a fresh game in Main phase, turn 1 (skips RPS/mulligan/setup).
    /// Pre-creates unique copy IDs for each card template so `id()` returns
    /// distinct IDs for per-copy modifier tracking without mutating state.
    pub fn new(db: Arc<CardDatabase>) -> Self {
        let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
        let p2 = Player::new("p2".into(), "Player 2".into(), false);
        p1.is_first_attacker = true;

        let mut state = GameState::new(p1, p2, db.clone());
        state.current_phase = Phase::Main;
        state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        state.turn_number = 1;

        // Pre-create copy IDs for every card template
        let mut copy_pool: HashMap<i16, Vec<i16>> = HashMap::new();
        let template_ids: Vec<i16> = state.card_database.cards.keys().copied().collect();
        for &tid in &template_ids {
            let mut copies = Vec::new();
            for _ in 0..5 {
                let cid = Arc::make_mut(&mut state.card_database).create_copy(tid);
                copies.push(cid);
            }
            copy_pool.insert(tid, copies);
        }

        // Use the mutated database (with copies) as the test's db reference
        let db_with_copies = state.card_database.clone();

        TestGame {
            db: db_with_copies,
            state,
            copy_pool: RefCell::new(copy_pool),
        }
    }

    /// Look up a card's numeric ID by card_no in the database.
    /// Returns the same unique copy_id for each card_no (not the base template_id).
    /// Each template gets 5 pre-created copies; `id()` returns the same copy_id
    /// for repeated calls with the same card_no so test patterns like
    /// `game.state.hand.cards.push(game.id("x"))` + `game.set_live_card(game.id("x"))`
    /// both refer to the same in-zone card. Use `game.new_id("...")` for distinct copies.
    pub fn id(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        let pool = self.copy_pool.borrow();
        pool.get(&template_id)
            .and_then(|v| v.last().copied())
            .unwrap_or(template_id)
    }

    /// Get a NEW unique copy_id (different from `id()`).
    /// Each call returns a distinct ID, used when multiple copies of the same card
    /// are needed in the same zone.
    pub fn new_id(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        self.copy_pool
            .borrow_mut()
            .get_mut(&template_id)
            .and_then(|v| v.pop())
            .unwrap_or(template_id)
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

    /// Try to activate ability, returning Result for error handling
    pub fn try_activate_ability(&mut self, stage_card_id: i16) -> Result<(), String> {
        TurnEngine::execute_main_phase_action(
            &mut self.state,
            &ActionType::UseAbility,
            Some(stage_card_id),
            None,
            None,
            None,
        )
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
            None,
            None,
            None,
            None,
        )
        .expect("pass failed");
    }

    // ---- Debugging ----

    /// Resolve a card ID to its name from the database.
    pub fn name(&self, id: i16) -> String {
        self.state
            .card_database
            .get_card(id)
            .map(|c| format!("{} ({})", c.name, c.card_no))
            .unwrap_or_else(|| format!("#{}", id))
    }

    /// Print card IDs in player1's hand.
    pub fn dbg_hand(&self) {
        let cards: Vec<String> = self
            .state
            .player1
            .hand
            .cards
            .iter()
            .map(|&id| self.name(id))
            .collect();
        eprintln!("[HAND] {:?}", cards);
    }

    /// Print card IDs in player1's waitroom.
    pub fn dbg_discard(&self) {
        let cards: Vec<String> = self
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .map(|&id| self.name(id))
            .collect();
        eprintln!("[DISCARD] {:?}", cards);
    }

    /// Print cards on player1's stage.
    pub fn dbg_stage(&self) {
        let cards: Vec<String> = self
            .state
            .player1
            .stage
            .stage
            .iter()
            .map(|&id| {
                if id == -1 {
                    "empty".into()
                } else {
                    self.name(id)
                }
            })
            .collect();
        eprintln!("[STAGE] {:?}", cards);
    }

    /// Get type of pending choice as a string.
    pub fn pending_choice_type(&self) -> Option<String> {
        if let Some(choice) = self.state.ability_queue.is_waiting_for_choice() {
            match choice {
                Choice::SelectCard { .. } => Some("SelectCard".to_string()),
                Choice::SelectTarget { .. } => Some("SelectTarget".to_string()),
                Choice::SelectPosition { .. } => Some("SelectPosition".to_string()),
                Choice::SelectHeartColor { .. } => Some("SelectHeartColor".to_string()),
                Choice::SelectHeartType { .. } => Some("SelectHeartType".to_string()),
            }
        } else if let Some(ref pc) = self.state.pending_choice {
            match pc["choice_type"].as_str() {
                Some("SelectCard") => Some("SelectCard".to_string()),
                Some("SelectTarget") => Some("SelectTarget".to_string()),
                Some("SelectPosition") => Some("SelectPosition".to_string()),
                Some("SelectHeartColor") => Some("SelectHeartColor".to_string()),
                Some("SelectHeartType") => Some("SelectHeartType".to_string()),
                _ => Some("Unknown".to_string()),
            }
        } else {
            None
        }
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
