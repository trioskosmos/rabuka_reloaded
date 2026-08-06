/// Shared test helpers for gameplay integration tests.
///
/// Every test loads the real card database, then uses `TestGame` to
/// set up a board state and play through a scenario.
///
/// Filler cards (zero abilities, no ability triggers) are available in
/// `tests/data/cards.json` and can be referenced by card_no.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
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

/// Embedded card data (compile-time, no file I/O at test startup).
static CARDS_JSON: &str = include_str!("../../../cards/cards.json");

struct PreloadedDb {
    db: CardDatabase,
    pool: HashMap<i16, Vec<i16>>,
}

static PRELOADED: OnceLock<PreloadedDb> = OnceLock::new();

/// Enable `log::debug!` output from the engine (env_logger). OFF by default;
/// turn it on for a specific test with e.g. `RUST_LOG=debug cargo test --test
/// run_all <failing_test> -- --nocapture`.
fn init_test_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default()).try_init();
}

/// Load (once) and return a pre-seeded database.
pub fn load_real_database() -> Arc<CardDatabase> {
    init_test_logger();
    PRELOADED.get_or_init(|| {
        let t0 = std::time::Instant::now();
        let cards =
            CardLoader::load_cards_from_strs(CARDS_JSON).expect("Failed to load embedded cards");
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
        let elapsed = t0.elapsed();
        eprintln!("[timing] db load: {}ms", elapsed.as_millis());
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

/// Push an energy_zone → energy_deck movement event, the trigger for
/// "…エネルギーがエネルギー置き場からエネルギーデッキに置かれたとき".
/// Uses the `-1` anonymous energy resource placeholder; the engine accepts it
/// for energy zone-change conditions (see resolve_moved_cards_source).
pub fn push_energy_zone_change(game: &mut TestGame, player: &str) {
    game.state
        .push_movement_event(-1, "energy_zone", "energy_deck", None, player, true);
}

/// Enable the engine's condition-verdict logger, run a TAS scan, and return a
/// human-readable tree of every condition verdict. Useful for debugging why an
/// ability did (or didn't) fire without editing engine source.
pub fn ability_verdicts(game: &mut TestGame, player: &str) -> String {
    use rabuka_engine::ability::debug::set_debug;
    use rabuka_engine::ability::log::{
        clear_verdicts, drain_verdicts, AbilityLogItem,
    };

    fn fmt(items: &[AbilityLogItem], indent: usize) -> String {
        let mut out = String::new();
        for it in items {
            match it {
                AbilityLogItem::Condition {
                    condition_type,
                    expectation,
                    actual,
                    passed,
                    children,
                    text,
                } => {
                    out.push_str(&"  ".repeat(indent));
                    out.push_str(&format!(
                        "[{}] {} => {} ({} / {}?)\n",
                        if *passed { "PASS" } else { "FAIL" },
                        condition_type,
                        actual,
                        expectation,
                        text
                    ));
                    out.push_str(&fmt(children, indent + 1));
                }
                AbilityLogItem::Cost {
                    text,
                    expectation,
                    actual,
                    passed,
                    optional,
                } => {
                    out.push_str(&"  ".repeat(indent));
                    out.push_str(&format!(
                        "[{}] cost '{}' {} vs {} (optional={})\n",
                        if *passed { "PASS" } else { "FAIL" },
                        text,
                        actual,
                        expectation,
                        optional
                    ));
                }
                AbilityLogItem::Effect { action, details, .. } => {
                    out.push_str(&"  ".repeat(indent));
                    out.push_str(&format!("[EFFECT] {} — {}\n", action, details));
                }
            }
        }
        out
    }

    set_debug(true);
    clear_verdicts();

    let pid = if player == "p1" || player == &game.state.player1.id {
        game.state.player1.id.clone()
    } else {
        game.state.player2.id.clone()
    };
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    // Capture scan-time condition verdicts BEFORE the resolver consumes the
    // buffer during effect processing.
    let scan_verdicts = drain_verdicts();
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
    // Capture resolution-time verdicts (effects re-evaluate can_activate_effect).
    let mut all = scan_verdicts;
    all.extend(drain_verdicts());
    set_debug(false);
    fmt(&all, 0)
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
    debug_enabled: bool,
    pool_positions: RefCell<HashMap<i16, usize>>,
    internal_counter: Cell<i16>,
    #[cfg(feature = "alloc_tracker")]
    _alloc_guard: Option<rabuka_engine::alloc_counter::AllocGuard>,
}

impl Drop for TestGame {
    fn drop(&mut self) {
        if self.debug_enabled {
            // Don't set_debug(false) here — ABILITY_DEBUG is a global AtomicBool
            // and turning it off in one test's Drop can disable it for another
            // test running in parallel. Just flush the log buffer.
            AbDebug::flush_to_rule_log(&mut self.state.rule_log);
        }
        // Dump structured log to stderr when RABUKA_RULE_LOG=1 is set.
        // Run tests with `--nocapture` to see output.
        if std::env::var("RABUKA_RULE_LOG").as_deref() == Ok("1")
            && !self.state.structured_log.is_empty()
        {
            let test_name = std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string();
            eprintln!("\n=== STRUCTURED LOG: {} ===", test_name);
            for entry in &self.state.structured_log {
                let meta_str = entry
                    .metadata
                    .as_ref()
                    .map(|m| serde_json::to_string(m).unwrap_or_default())
                    .unwrap_or_else(|| "null".to_string());
                eprintln!("[{}] {}", entry.category, entry.text);
                if meta_str != "null" {
                    eprintln!("  {}", meta_str);
                }
            }
            eprintln!("=== END STRUCTURED LOG ===\n");
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
        // cargo test: all gates ON (loud). cargo t: detects --test-threads → all OFF.
        let quiet = std::env::args().any(|a| a.starts_with("--test-threads"));
        if !quiet {
            let _ = env_logger::try_init();
            rabuka_engine::ability::debug::set_debug(true);
        }
        let debug_enabled = !quiet || std::env::var("RABUKA_DEBUG").is_ok();

        TestGame {
            db: state.card_database.clone(),
            state,
            debug_enabled,
            pool_positions: RefCell::new(HashMap::new()),
            internal_counter: Cell::new(20000),
            #[cfg(feature = "alloc_tracker")]
            _alloc_guard: rabuka_engine::alloc_counter::start(),
        }
    }

    /// Look up a card's numeric ID by card_no in the database.
    /// Returns a new unique copy_id each call.
    /// Each call returns a distinct ID so multiple copies of the same card on stage
    /// get per-card modifier tracking instead of sharing modifiers.
    /// Store the result in a variable if you need to reference the same card later.
    pub fn id(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        let pool = &PRELOADED.get().unwrap().pool;
        let mut positions = self.pool_positions.borrow_mut();
        let pos = positions.entry(template_id).or_insert(0);
        let id = pool
            .get(&template_id)
            .and_then(|v| v.get(*pos).copied())
            .unwrap_or(template_id);
        *pos += 1;
        id
    }

    /// Get a NEW unique copy_id (different from `id()` and `id_ref()`).
    /// Unlike `id()`, this uses the template ID when available or an
    /// internal counter when more copies are needed than the pool provides.
    pub fn new_id(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        let pool = &PRELOADED.get().unwrap().pool;
        let mut positions = self.pool_positions.borrow_mut();
        let pos = positions.entry(template_id).or_insert(0);
        let id = pool
            .get(&template_id)
            .and_then(|v| v.get(*pos).copied())
            .unwrap_or_else(|| {
                let cid = self.internal_counter.get();
                self.internal_counter.set(cid + 1);
                cid
            });
        *pos += 1;
        id
    }

    /// Get a stable reference ID for a card_no (always returns the same copy).
    /// Unlike `id()`, this peeks without consuming the pool entry.
    /// Use this when you need a known ID for assertions or lookups
    /// after the card has been placed in a zone via `id()`.
    pub fn id_ref(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        let pool = &PRELOADED.get().unwrap().pool;
        let positions = self.pool_positions.borrow();
        let pos = positions.get(&template_id).copied().unwrap_or(0);
        pool.get(&template_id)
            .and_then(|v| v.get(pos).copied())
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
        self.state.player1.energy_zone.add_active(count as u8);
    }

    /// Dump structured_log entries to stderr for inspection.
    /// Run tests with `--nocapture` to see output.
    pub fn dump_structured_log(&self) {
        for entry in &self.state.structured_log {
            let meta_str = entry
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
                .unwrap_or_else(|| "null".to_string());
            eprintln!(
                "[LOG cat={} turn={} player={} card={:?} name={:?}] {}",
                entry.category,
                entry.turn,
                entry.player_label,
                entry.source_card_id,
                entry.source_card_name,
                entry.text
            );
            if meta_str != "null" {
                eprintln!("  metadata: {}", meta_str);
            }
        }
    }

    /// Dump structured_log entries for a specific category.
    pub fn dump_structured_log_category(&self, category: &str) {
        for entry in &self.state.structured_log {
            if entry.category != category {
                continue;
            }
            let meta_str = entry
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string_pretty(m).unwrap_or_default())
                .unwrap_or_else(|| "null".to_string());
            eprintln!(
                "[LOG cat={} turn={} player={} card={:?} name={:?}] {}",
                entry.category,
                entry.turn,
                entry.player_label,
                entry.source_card_id,
                entry.source_card_name,
                entry.text
            );
            if meta_str != "null" {
                eprintln!("  metadata: {}", meta_str);
            }
        }
    }

    /// Dump rule_log entries to stderr for inspection.
    pub fn dump_rule_log(&self) {
        for line in &self.state.rule_log {
            eprintln!("[RULE_LOG] {}", line);
        }
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
        if let Err(_) = self.select_via_generated(indices) {
            TurnEngine::resume_with_choice(&mut self.state, None, Some(indices.to_vec()))
                .expect("select_indices failed");
        }
    }

    /// Answer a pending SelectCard choice over the energy zone by selecting the
    /// first `count` energy cards (the under-member placement choice). Panics if
    /// the pending choice is not an energy SelectCard or if there aren't enough
    /// energy cards to select.
    pub fn select_energy_from_zone(&mut self, count: usize) {
        let idxs: Vec<usize> = (0..count).collect();
        self.select_indices(&idxs);
    }

    /// Try to select indices, returning the error instead of panicking.
    pub fn try_select_indices(&mut self, indices: &[usize]) -> Result<(), String> {
        if let Err(_) = self.select_via_generated(indices) {
            TurnEngine::resume_with_choice(&mut self.state, None, Some(indices.to_vec()))
        } else {
            Ok(())
        }
    }

    /// For SelectCard choices, find the generated action matching the given indices
    /// and resolve through it — matching what the real frontend does.
    /// Falls back for non-SelectCard choices and multi-index selections.
    fn select_via_generated(&mut self, indices: &[usize]) -> Result<(), String> {
        let is_select_card = self
            .state
            .get_pending_choice()
            .is_some_and(|c| matches!(c, rabuka_engine::ability::types::Choice::SelectCard { .. }));
        if !is_select_card {
            return Err("Not a SelectCard choice".into());
        }
        let actions = rabuka_engine::game_setup::generate_possible_actions(&self.state);
        if indices.is_empty() {
            let skip = actions
                .iter()
                .find(|a| a.action_type == ActionType::ChoiceSkip)
                .ok_or("No skip action available")?;
            let p = skip
                .parameters
                .as_ref()
                .ok_or("Skip action has no params")?;
            TurnEngine::resume_with_choice(&mut self.state, p.card_id, p.card_indices.clone())
        } else if indices.len() == 1 {
            let action = actions
                .iter()
                .find(|a| {
                    a.action_type == ActionType::ChoiceSelect
                        && a.parameters
                            .as_ref()
                            .and_then(|p| p.card_indices.as_deref())
                            == Some(indices)
                })
                .ok_or_else(|| {
                    format!(
                        "No ChoiceSelect action with card_indices={:?}. Available: {:?}",
                        indices,
                        actions
                            .iter()
                            .filter(|a| a.action_type == ActionType::ChoiceSelect)
                            .map(|a| a
                                .parameters
                                .as_ref()
                                .and_then(|p| p.card_indices.as_deref()))
                            .collect::<Vec<_>>()
                    )
                })?;
            let p = action.parameters.as_ref().ok_or("Action has no params")?;
            TurnEngine::resume_with_choice(&mut self.state, p.card_id, p.card_indices.clone())
        } else {
            Err("Multi-index not supported via action path".into())
        }
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
            self.select_indices(&[idx]);
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
                self.select_indices(&[]);
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
                if target == "position|destination" || target == "area_select" =>
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
                if target == "position|destination" || target == "area_select" =>
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

    /// Get the `count` from a pending SelectCard choice.
    pub fn pending_choice_count(&self) -> usize {
        if let Some(choice) = self.state.ability_queue.is_waiting_for_choice() {
            if let Choice::SelectCard { count, .. } = choice {
                return *count;
            }
        } else if let Some(ref pc) = self.state.get_pending_choice_json() {
            if let Some(count) = pc["count"].as_u64() {
                return count as usize;
            }
        }
        0
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
        let actual = self.state.player1.energy_zone.active_count();
        assert_eq!(
            actual, expected as u8,
            "{}: expected {} energy, got {}",
            msg, expected, actual
        );
    }

    /// Assert the pending choice JSON has the expected card_id and card_name.
    /// This validates the identity metadata the frontend uses to render the header.
    /// Panics if there's no pending choice or the fields don't match.
    pub fn assert_choice_identity(
        &self,
        expected_card_id: i16,
        expected_card_name: &str,
        expected_player_id: &str,
    ) {
        let json = self
            .state
            .get_pending_choice_json()
            .expect("No pending choice JSON available");
        let actual_id = json["card_id"].as_i64().map(|v| v as i16);
        assert_eq!(
            actual_id,
            Some(expected_card_id),
            "choice JSON card_id: expected {:?}, got {:?}",
            expected_card_id,
            actual_id
        );
        let actual_name = json["card_name"].as_str().unwrap_or("");
        assert_eq!(
            actual_name, expected_card_name,
            "choice JSON card_name: expected '{}', got '{}'",
            expected_card_name, actual_name
        );
        let actual_pid = json["choice_player_id"].as_str().unwrap_or("");
        assert_eq!(
            actual_pid, expected_player_id,
            "choice JSON choice_player_id: expected '{}', got '{}'",
            expected_player_id, actual_pid
        );
    }

    /// Assert the pending choice JSON's selection_cards contain a card with the
    /// given card_no and name.  Panics if the card isn't found.
    pub fn assert_selection_contains(&self, expected_card_no: &str, expected_name: &str) {
        let json = self
            .state
            .get_pending_choice_json()
            .expect("No pending choice JSON available");
        let cards = json["selection_cards"]
            .as_array()
            .expect("No selection_cards in choice JSON");
        let found = cards.iter().any(|c| {
            c["card_no"].as_str() == Some(expected_card_no)
                && c["name"].as_str() == Some(expected_name)
        });
        assert!(
            found,
            "selection_cards should contain card_no='{}' name='{}', but it doesn't. Cards: {:?}",
            expected_card_no, expected_name, cards
        );
    }

    /// Assert the pending choice JSON's selection_cards do NOT contain the given card_no.
    pub fn assert_selection_not_contains(&self, card_no: &str) {
        let json = self
            .state
            .get_pending_choice_json()
            .expect("No pending choice JSON available");
        let cards = json["selection_cards"]
            .as_array()
            .expect("No selection_cards in choice JSON");
        let found = cards.iter().any(|c| c["card_no"].as_str() == Some(card_no));
        assert!(
            !found,
            "selection_cards should NOT contain '{}', but it does",
            card_no
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
