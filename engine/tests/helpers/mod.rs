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
use std::sync::{Arc, OnceLock};

use rabuka_engine::ability::debug::AbDebug;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;

/// Pre-seeded database + copy pool, initialised once per process.
/// The database has 5 copies per template pre-created and registered,
/// so tests never need to mutate the `Arc<CardDatabase>`.
struct PreloadedDb {
    db: CardDatabase,
    pool: HashMap<i16, Vec<i16>>,
}

static PRELOADED: OnceLock<PreloadedDb> = OnceLock::new();

/// Load (once) and return a pre-seeded database with 5 copies per template.
pub fn load_real_database() -> Arc<CardDatabase> {
    PRELOADED.get_or_init(|| {
        let cards_path = Path::new("../cards/cards.json");
        let cards = CardLoader::load_cards_from_file(cards_path)
            .expect("Failed to load real cards from ../cards/cards.json");
        let mut db = CardDatabase::load_or_create(cards);
        let tids: Vec<i16> = db.cards.keys().copied().collect();
        let mut pool: HashMap<i16, Vec<i16>> = HashMap::new();
        for &tid in &tids {
            let mut v = Vec::with_capacity(5);
            for _ in 0..5 {
                v.push(db.create_copy(tid));
            }
            pool.insert(tid, v);
        }
        PreloadedDb { db, pool }
    });
    static DB: OnceLock<Arc<CardDatabase>> = OnceLock::new();
    DB.get_or_init(|| Arc::new(PRELOADED.get().unwrap().db.clone()))
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
    debug_enabled: bool,
    copy_counter: RefCell<i16>,
}

impl Drop for TestGame {
    fn drop(&mut self) {
        if self.debug_enabled {
            // Don't set_debug(false) here — ABILITY_DEBUG is a global AtomicBool
            // and turning it off in one test's Drop can disable it for another
            // test running in parallel. Just flush the log buffer.
            AbDebug::flush_to_rule_log(&mut self.state.rule_log);
        }
    }
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

        let mut state = GameState::new(p1, p2, db);
        state.current_phase = Phase::Main;
        state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        state.turn_number = 1;
        rabuka_engine::ability::debug::set_debug(true);

        // Clone the pre-seeded copy pool (once-populated, cheap per-test clone)
        let copy_pool = PRELOADED.get().unwrap().pool.clone();

        TestGame {
            db: state.card_database.clone(),
            state,
            copy_pool: RefCell::new(copy_pool),
            debug_enabled: true,
            copy_counter: RefCell::new(20000),
        }
    }

    /// Look up a card's numeric ID by card_no in the database.
    /// Returns a new unique copy_id each call (popped from the pre-created pool).
    /// Each call returns a distinct ID so multiple copies of the same card on stage
    /// get per-card modifier tracking instead of sharing modifiers.
    /// Store the result in a variable if you need to reference the same card later.
    pub fn id(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        self.copy_pool
            .borrow_mut()
            .get_mut(&template_id)
            .and_then(|v| v.pop())
            .unwrap_or(template_id)
    }

    /// Get a NEW unique copy_id (different from `id()` and `id_ref()`).
    /// Returns the template ID directly, matching the original behavior when
    /// the 1-copy-per-template pool was empty after `id()` consumed it.
    /// This preserves the original semantic: `new_id()` provides an ID that's
    /// distinct from `id()` for the same card, but multiple `new_id()` calls
    /// for the same card may return the same template ID.
    pub fn new_id(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        self.copy_pool
            .borrow_mut()
            .get_mut(&template_id)
            .and_then(|v| v.pop())
            .unwrap_or_else(|| {
                let cid = *self.copy_counter.borrow();
                *self.copy_counter.borrow_mut() = cid + 1;
                cid
            })
    }

    /// Get a stable reference ID for a card_no (always returns the same copy).
    /// Unlike `id()`, this peeks without consuming the pool entry.
    /// Use this when you need a known ID for assertions or lookups
    /// after the card has been placed in a zone via `id()`.
    pub fn id_ref(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        let pool = self.copy_pool.borrow();
        pool.get(&template_id)
            .and_then(|v| v.last().copied())
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
    /// Each energy card gets its own unique copy ID.
    pub fn give_energy(&mut self, count: usize) {
        for _ in 0..count {
            let energy_card = self.id("LL-E-001-SD");
            self.state.player1.energy_zone.cards.push(energy_card);
        }
        self.state.player1.energy_zone.active_energy_count += count;
    }

    // ---- Actions ----

    /// Play a member card from hand onto the stage.
    pub fn play_to_stage(&mut self, card_id: i16, area: MemberArea) {
        self.try_play_to_stage(card_id, area)
            .expect("play_to_stage failed");
    }

    /// Attempt to play a member card from hand onto the stage. Returns Result.
    pub fn try_play_to_stage(&mut self, card_id: i16, area: MemberArea) -> Result<(), String> {
        TurnEngine::execute_main_phase_action(
            &mut self.state,
            &ActionType::PlayMemberToStage,
            Some(card_id),
            None,
            Some(area),
            Some(false),
        )
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
        self.state.has_pending_choice()
    }

    /// Return the pending choice for inspection, panics if none.
    pub fn get_pending_choice(&self) -> &Choice {
        self.state.get_pending_choice().expect("No pending choice")
    }

    /// Assert the pending choice is a SelectCard matching the given criteria.
    pub fn assert_select_card(
        &self,
        expected_zone: &str,
        expected_count: usize,
        expected_allow_skip: bool,
    ) {
        let choice = self.get_pending_choice();
        match choice {
            Choice::SelectCard {
                zone,
                count,
                allow_skip,
                ..
            } => {
                assert_eq!(zone, expected_zone, "SelectCard zone mismatch");
                assert_eq!(*count, expected_count, "SelectCard count mismatch");
                assert_eq!(
                    *allow_skip, expected_allow_skip,
                    "SelectCard allow_skip mismatch"
                );
            }
            _ => panic!("Expected SelectCard, got {:?}", choice),
        }
    }

    /// Assert the pending choice is a SelectTarget for conditional_optional.
    pub fn assert_conditional_optional(&self, expected_opts: &[&str]) {
        let choice = self.get_pending_choice();
        match choice {
            Choice::SelectTarget {
                target, options, ..
            } => {
                assert_eq!(
                    target, "conditional_optional",
                    "Expected conditional_optional target"
                );
                if let Some(ref opts) = options {
                    assert_eq!(opts.len(), expected_opts.len(), "Option count mismatch");
                    for (i, expected) in expected_opts.iter().enumerate() {
                        assert_eq!(&opts[i], expected, "Option {} mismatch", i);
                    }
                }
            }
            _ => panic!(
                "Expected SelectTarget(conditional_optional), got {:?}",
                choice
            ),
        }
    }

    /// Print the current ability queue state to stderr (for test debug).
    pub fn dump_queue(&self) {
        let state_str = self.state.ability_queue.dump_state();
        eprintln!("[QUEUE_DUMP]\n{}", state_str);
    }

    /// Select cards by waitroom/hand indices (for SelectCard choices).
    pub fn select_indices(&mut self, indices: &[usize]) {
        TurnEngine::resume_with_choice(&mut self.state, None, Some(indices.to_vec()))
            .expect("select_indices failed");
    }

    /// Find a card in the waitroom by ID and select it using the filtered-relative
    /// index (position within filtered_indices), which is exactly what the frontend
    /// sends. This correctly simulates the real-game selection protocol.
    /// Panics if the card isn't found or isn't in filtered_indices.
    pub fn select_waitroom_card_filtered(&mut self, card_id: i16) {
        let pending = self.get_pending_choice().clone();
        let (fi, zone) = match &pending {
            Choice::SelectCard { filtered_indices: Some(fi), zone, .. } => (fi.clone(), zone.clone()),
            _ => panic!(
                "select_waitroom_card_filtered: expected SelectCard with filtered_indices, got {:?}",
                pending
            ),
        };
        if zone != "discard" && zone != "waitroom" {
            panic!(
                "select_waitroom_card_filtered: expected discard zone, got '{}'",
                zone
            );
        }
        let pos = self
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .position(|&c| c == card_id)
            .expect("Card not found in waitroom");
        let filtered_idx = fi
            .iter()
            .position(|&p| p == pos)
            .unwrap_or_else(|| panic!("Card not in filtered_indices: pos={} fi={:?}", pos, fi));
        self.select_indices(&[filtered_idx]);
    }

    /// Try to select indices, returning the error instead of panicking.
    pub fn try_select_indices(&mut self, indices: &[usize]) -> Result<(), String> {
        TurnEngine::resume_with_choice(&mut self.state, None, Some(indices.to_vec()))
    }

    /// Select one index at a time for any-number choices (e.g. reveal any number).
    /// If the pending choice is any-number (count=0, allow_skip=true), feeds each
    /// index individually, waiting for re-prompts between picks, then sends an
    /// empty selection to finalize.  For fixed-count choices it falls back to
    /// the regular multi-index select_indices.
    pub fn select_indices_sequential(&mut self, indices: &[usize]) {
        if indices.is_empty() {
            self.select_indices(indices);
            return;
        }
        // Check if the pending choice is any-number
        let is_any_number = self.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 0,
                    allow_skip: true,
                    ..
                }
            )
        });
        if !is_any_number {
            // Fixed-count: send all at once (same as regular select_indices)
            self.select_indices(indices);
            return;
        }
        // Any-number: feed one at a time, handling re-prompts
        for (i, &idx) in indices.iter().enumerate() {
            TurnEngine::resume_with_choice(&mut self.state, None, Some(vec![idx])).unwrap_or_else(
                |e| panic!("select_indices_sequential failed at index {}: {}", i, e),
            );
            // Expect a re-prompt after each selection except the last
            // (when all cards are taken, the engine auto-finalizes).
            if i + 1 < indices.len() {
                assert!(
                    self.state.has_pending_choice(),
                    "Expected re-prompt after selecting index {} of {}",
                    i + 1,
                    indices.len()
                );
            }
        }
        // If a re-prompt is still pending after the last pick, skip to finalize
        // (this happens when the hand has more cards than were selected).
        if self.state.has_pending_choice() {
            let still_any = self.state.get_pending_choice().is_some_and(|c| {
                matches!(
                    c,
                    rabuka_engine::ability::types::Choice::SelectCard {
                        count: 0,
                        allow_skip: true,
                        ..
                    }
                )
            });
            if still_any {
                TurnEngine::resume_with_choice(&mut self.state, None, Some(vec![]))
                    .expect("select_indices_sequential skip failed");
            }
        }
    }

    /// Select a choice option by index (for SelectTarget choices like answers, alternatives).
    pub fn select_option(&mut self, option_index: i16) {
        TurnEngine::resume_with_choice(&mut self.state, Some(option_index), None)
            .expect("select_option failed");
    }

    /// Generate the list of legal actions the frontend would show for the current
    /// pending choice.  For `position|destination` targets, filters to only
    /// `ChoicePosition` actions (the actual position buttons the user sees).
    pub fn generated_actions(&self) -> Vec<rabuka_engine::game_setup::Action> {
        let pending = self.get_pending_choice();
        let all = rabuka_engine::game_setup::generate_possible_actions(&self.state);
        match pending {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "position|destination" =>
            {
                all.into_iter()
                    .filter(|a| {
                        a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition
                    })
                    .collect()
            }
            _ => all,
        }
    }

    /// Select the Nth generated action (simulates clicking the Nth button the
    /// frontend would show).  Panics with the list of labels if out of range.
    pub fn select_generated(&mut self, nth: usize) {
        let pending = self.get_pending_choice().clone();
        let all = rabuka_engine::game_setup::generate_possible_actions(&self.state);
        let matching: Vec<&rabuka_engine::game_setup::Action> = match &pending {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "position|destination" =>
            {
                all.iter()
                    .filter(|a| {
                        a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition
                    })
                    .collect()
            }
            _ => all.iter().collect(),
        };
        assert!(
            nth < matching.len(),
            "select_generated({}): only {} generated actions",
            nth,
            matching.len(),
        );
        let action = matching[nth];
        TurnEngine::resume_with_choice(
            &mut self.state,
            action.parameters.as_ref().and_then(|p| p.card_id),
            action
                .parameters
                .as_ref()
                .and_then(|p| p.card_indices.clone()),
        )
        .expect("select_generated failed");
    }

    /// Drain all auto-ability choice prompts, selecting the first option each time.
    pub fn drain_auto_ability_choices(&mut self) {
        while let Some(choice) = self.state.get_pending_choice() {
            match choice {
                Choice::SelectAutoAbility { .. } => {
                    self.select_indices(&[]);
                }
                _ => break,
            }
        }
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
                Choice::SelectAutoAbility { .. } => Some("SelectAutoAbility".to_string()),
                Choice::SelectLiveSuccess { .. } => Some("SelectLiveSuccess".to_string()),
            }
        } else if let Some(ref pc) = self.state.get_pending_choice_json() {
            match pc["choice_type"].as_str() {
                Some("SelectCard") => Some("SelectCard".to_string()),
                Some("SelectTarget") => Some("SelectTarget".to_string()),
                Some("SelectPosition") => Some("SelectPosition".to_string()),
                Some("SelectHeartColor") => Some("SelectHeartColor".to_string()),
                Some("SelectHeartType") => Some("SelectHeartType".to_string()),
                Some("SelectAutoAbility") => Some("SelectAutoAbility".to_string()),
                Some("SelectLiveSuccess") => Some("SelectLiveSuccess".to_string()),
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
        } else if let Some(ref pc) = self.state.get_pending_choice_json() {
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

    /// Print the execution trace from the last ability resolution.
    pub fn print_trace(&self) {
        if let Some(ref trace) = self.state.last_ability_trace {
            Self::print_trace_node(trace, 0);
        } else {
            eprintln!("[TRACE] no trace available (no ability resolved yet)");
        }
    }

    fn print_trace_node(node: &rabuka_engine::ability::types::AbilityTraceNode, depth: usize) {
        let pad = "  ".repeat(depth);
        let card_info = node
            .card
            .as_deref()
            .map(|c| format!(" [{}]", c))
            .unwrap_or_default();
        eprintln!("{}{}▸ {}{}", pad, depth, node.label, card_info);
        if let Some(ref before) = node.before {
            eprintln!(
                "{}  before: hand={} stage={} waitroom={} energy={} active_energy={} deck={}",
                pad,
                before.hand_count,
                before.stage_count,
                before.waitroom_count,
                before.energy_count,
                before.active_energy_count,
                before.deck_count,
            );
        }
        if let Some(ref after) = node.after {
            eprintln!(
                "{}  after:  hand={} stage={} waitroom={} energy={} active_energy={} deck={}",
                pad,
                after.hand_count,
                after.stage_count,
                after.waitroom_count,
                after.energy_count,
                after.active_energy_count,
                after.deck_count,
            );
        }
        for child in &node.children {
            Self::print_trace_node(child, depth + 1);
        }
    }

    /// Assert the trace contains a node whose label matches the given substring.
    pub fn assert_trace_contains(&self, pattern: &str, msg: &str) {
        let trace = self
            .state
            .last_ability_trace
            .as_ref()
            .expect("assert_trace_contains: no trace available");
        let found = Self::trace_node_matches(trace, pattern);
        assert!(
            found,
            "{}: expected trace to contain '{}', but no node matched.\n{}",
            msg,
            pattern,
            self.format_trace_string(),
        );
    }

    /// Assert the trace does NOT contain a node whose label matches the given substring.
    pub fn assert_trace_not_contains(&self, pattern: &str, msg: &str) {
        if let Some(ref trace) = self.state.last_ability_trace {
            let found = Self::trace_node_matches(trace, pattern);
            assert!(
                !found,
                "{}: expected trace NOT to contain '{}', but a node matched.\n{}",
                msg,
                pattern,
                self.format_trace_string(),
            );
        }
    }

    fn trace_node_matches(
        node: &rabuka_engine::ability::types::AbilityTraceNode,
        pattern: &str,
    ) -> bool {
        node.label.contains(pattern)
            || node.card.as_deref().map_or(false, |c| c.contains(pattern))
            || node
                .children
                .iter()
                .any(|c| Self::trace_node_matches(c, pattern))
    }

    fn format_trace_string(&self) -> String {
        let mut buf = String::new();
        if let Some(ref trace) = self.state.last_ability_trace {
            Self::format_trace_node(trace, 0, &mut buf);
        } else {
            buf.push_str("  (no trace)\n");
        }
        buf
    }

    fn format_trace_node(
        node: &rabuka_engine::ability::types::AbilityTraceNode,
        depth: usize,
        buf: &mut String,
    ) {
        let pad = "  ".repeat(depth);
        let card_info = node
            .card
            .as_deref()
            .map(|c| format!(" [{}]", c))
            .unwrap_or_default();
        buf.push_str(&format!("{}{}▸ {}{}\n", pad, depth, node.label, card_info));
        for child in &node.children {
            Self::format_trace_node(child, depth, buf);
        }
    }

    /// Assert hand contains exactly n cards.
    pub fn assert_hand(&self, expected: usize, msg: &str) {
        let actual = self.state.player1.hand.len();
        assert_eq!(
            actual, expected,
            "{}: expected {} cards in hand, got {}",
            msg, expected, actual
        );
    }

    /// Assert stage position contains the given card.
    pub fn assert_stage_pos(&self, pos: MemberArea, card_id: i16, msg: &str) {
        let actual = self.state.player1.stage.get_area(pos);
        assert_eq!(
            actual,
            Some(card_id),
            "{}: expected {:?} at position {:?}, got {:?}",
            msg,
            self.state.card_database.get_card(card_id).map(|c| &c.name),
            pos,
            actual.and_then(|id| self.state.card_database.get_card(id).map(|c| &c.name))
        );
    }

    /// Assert energy count equals expected value.
    pub fn assert_energy(&self, expected: u32, msg: &str) {
        let actual = self.state.player1.energy_zone.active_energy_count;
        assert_eq!(
            actual, expected as usize,
            "{}: expected {} energy, got {}",
            msg, expected, actual
        );
    }

    /// Assert pending choice type matches expected variant name.
    pub fn assert_pending_choice_type(&self, expected: &str, msg: &str) {
        if let Some(choice) = self.state.ability_queue.is_waiting_for_choice() {
            let actual = match choice {
                Choice::SelectCard { .. } => "SelectCard",
                Choice::SelectTarget { .. } => "SelectTarget",
                Choice::SelectPosition { .. } => "SelectPosition",
                Choice::SelectHeartColor { .. } => "SelectHeartColor",
                Choice::SelectHeartType { .. } => "SelectHeartType",
                Choice::SelectAutoAbility { .. } => "SelectAutoAbility",
                Choice::SelectLiveSuccess { .. } => "SelectLiveSuccess",
            };
            assert_eq!(
                actual, expected,
                "{}: expected choice type {}, got {}",
                msg, expected, actual
            );
        } else {
            panic!(
                "{}: expected pending choice of type {}, got none",
                msg, expected
            );
        }
    }
}
