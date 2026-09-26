use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

mod assertions;
mod choices;
mod diagnostics;
mod trace;

#[macro_export]
macro_rules! assert_ability {
    ($game:expr, $player:expr, $cond:expr $(,)?) => {{
        if !($cond) {
            let __v = $crate::helpers::ability_verdicts(&mut $game, $player);
            panic!(
                "ability assertion failed: {}\n--- verdicts ---\n{}",
                stringify!($cond),
                __v
            );
        }
    }};
    ($game:expr, $player:expr, $cond:expr, $($arg:tt)+) => {{
        if !($cond) {
            let __v = $crate::helpers::ability_verdicts(&mut $game, $player);
            panic!(
                "{}\n--- verdicts ---\n{}",
                format!($($arg)+),
                __v
            );
        }
    }};
}

pub use choices::answer_choice;
pub use diagnostics::ability_verdicts;
pub(crate) use assert_ability;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::types::{Phase, TurnPhase};
use rabuka_engine::zones::MemberArea;

static CARDS_JSON: &str = include_str!("../../../cards/cards.json");

struct PreloadedDb {
    db: CardDatabase,
    pool: HashMap<i16, Vec<i16>>,
}

static PRELOADED: OnceLock<PreloadedDb> = OnceLock::new();

fn init_test_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default()).try_init();
}

fn start_test_watchdog() {
    static WATCHDOG: OnceLock<()> = OnceLock::new();
    WATCHDOG.get_or_init(|| {
        let secs: u64 = std::env::var("RABUKA_TEST_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(secs));
            eprintln!(
                "\n!!! TEST WATCHDOG: exceeded {secs}s (RABUKA_TEST_TIMEOUT_SECS) — \
                 a test is hung; aborting the test binary !!!"
            );
            std::process::exit(101);
        });
    });
}

pub fn load_real_database() -> Arc<CardDatabase> {
    init_test_logger();
    start_test_watchdog();
    PRELOADED.get_or_init(|| {
        let t0 = std::time::Instant::now();
        let cards =
            CardLoader::load_cards_from_strs(CARDS_JSON).expect("Failed to load embedded cards");
        let mut db = CardDatabase::load_or_create(cards);
        let tids: Vec<i16> = db.cards.keys().copied().collect();
        let mut pool: HashMap<i16, Vec<i16>> = HashMap::new();
        for &tid in &tids {
            let mut v = Vec::with_capacity(11);
            for _ in 0..11 {
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

pub fn card_id(db: &CardDatabase, card_no: &str) -> i16 {
    db.get_card_id(card_no)
        .unwrap_or_else(|| panic!("Card {card_no} not found in database"))
}

pub fn push_energy_zone_change(game: &mut TestGame, player: &str) {
    game.state
        .push_movement_event(-1, "energy_zone", "energy_deck", None, player, true);
}

pub fn fire_trigger(
    game: &mut TestGame,
    cid: i16,
    trigger: rabuka_engine::core::types::AbilityTrigger,
    trig: &str,
) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some(trig))
            .unwrap_or_else(|| panic!("card {} lacks a '{trig}' ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        ability_id,
        trigger,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

pub fn scan_autos_both(game: &mut TestGame) {
    for pid in [
        game.state.player1.id.clone(),
        game.state.player2.id.clone(),
    ] {
        game.state.trigger_auto_abilities_for_player(&pid);
        game.state.process_pending_auto_abilities(&pid);
    }
    if game.has_pending_choice() {
        answer_choice(game, 0);
    }
    game.drain_auto_ability_choices();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    P1,
    P2,
}

pub struct TestGame {
    pub db: Arc<CardDatabase>,
    pub state: GameState,
    debug_enabled: bool,
    pool_positions: RefCell<HashMap<i16, usize>>,
    trace: trace::Trace,
    #[cfg(feature = "alloc_tracker")]
    _alloc_guard: Option<rabuka_engine::alloc_counter::AllocGuard>,
}

#[allow(dead_code)]
impl TestGame {
    pub fn new(db: Arc<CardDatabase>) -> Self {
        let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
        let p2 = Player::new("p2".into(), "Player 2".into(), false);
        p1.is_first_attacker = true;

        let mut state = GameState::new(p1, p2, db);
        state.current_phase = Phase::Main;
        state.current_turn_phase = TurnPhase::FirstAttackerNormal;
        state.turn_number = 1;
        let quiet = std::env::var("RABUKA_QUIET").as_deref() == Ok("1");
        if !quiet {
            rabuka_engine::ability::debug::set_debug(true);
        }
        let debug_enabled = !quiet || std::env::var("RABUKA_DEBUG").is_ok();

        let game = TestGame {
            db: state.card_database.clone(),
            state,
            debug_enabled,
            pool_positions: RefCell::new(HashMap::new()),
            trace: trace::Trace::new(),
            #[cfg(feature = "alloc_tracker")]
            _alloc_guard: rabuka_engine::alloc_counter::start(),
        };
        if game.trace.live() {
            let t = std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string();
            game.trace.emit(format!("# test {}", t));
            trace::dump_state(&game, &game.trace);
        }
        game
    }

    fn consume_id(&self, card_no: &str, fallback: impl FnOnce(i16) -> i16) -> i16 {
        let template_id = card_id(&self.db, card_no);
        let pool = &PRELOADED.get().unwrap().pool;
        let mut positions = self.pool_positions.borrow_mut();
        let pos = positions.entry(template_id).or_insert(0);
        let id = pool
            .get(&template_id)
            .and_then(|v| v.get(*pos).copied())
            .unwrap_or_else(|| fallback(template_id));
        *pos += 1;
        id
    }

    pub fn id(&self, card_no: &str) -> i16 {
        self.consume_id(card_no, |template_id| template_id)
    }

    pub fn new_id(&self, card_no: &str) -> i16 {
        self.consume_id(card_no, |template_id| {
            // Past the 11-copy pool: fall back to the shared template id
            // (same as id()). Always real card data, never a collision —
            // at the cost of aliasing: instances past 11 are NOT distinct.
            // Bulk deck filler must use game.id() (shared template) anyway;
            // staged/played/tracked cards must stay within pool range.
            template_id
        })
    }

    pub fn id_ref(&self, card_no: &str) -> i16 {
        let template_id = card_id(&self.db, card_no);
        let pool = &PRELOADED.get().unwrap().pool;
        let positions = self.pool_positions.borrow();
        let pos = positions.get(&template_id).copied().unwrap_or(0);
        pool.get(&template_id)
            .and_then(|v| v.get(pos).copied())
            .unwrap_or(template_id)
    }

    pub fn set_active_side(&mut self, side: Side) {
        let (p1f, p2f) = match side {
            Side::P1 => (true, false),
            Side::P2 => (false, true),
        };
        self.state.player1.is_first_attacker = p1f;
        self.state.player2.is_first_attacker = p2f;
    }

    fn player_for(&mut self, side: Side) -> &mut Player {
        match side {
            Side::P1 => &mut self.state.player1,
            Side::P2 => &mut self.state.player2,
        }
    }

    pub fn add_to_hand_for(&mut self, side: Side, id: i16) {
        self.player_for(side).hand.cards.push(id);
    }

    pub fn give_energy_for(&mut self, side: Side, count: usize) {
        for _ in 0..count {
            let energy_card = self.id("LL-E-001-SD");
            // Insert at the active boundary so the positional convention
            // (prefix = active) holds even when waited energy exists.
            self.player_for(side).energy_zone.push_active(energy_card);
        }
    }

    pub fn try_play_to_stage_for(
        &mut self,
        side: Side,
        card_id: i16,
        area: MemberArea,
    ) -> Result<(), String> {
        self.set_active_side(side);
        self.run_main_action(
            None,
            &ActionType::PlayMemberToStage,
            Some(card_id),
            Some(area),
            None,
        )
    }

    pub fn activate_ability_for(&mut self, side: Side, stage_card_id: i16) -> Result<(), String> {
        self.set_active_side(side);
        self.run_main_action(
            None,
            &ActionType::UseAbility,
            Some(stage_card_id),
            None,
            None,
        )
    }

    pub fn player(&mut self) -> &mut Player {
        &mut self.state.player1
    }

    pub fn add_to_hand(&mut self, id: i16) {
        self.add_to_hand_for(Side::P1, id);
    }

    pub fn add_to_discard(&mut self, id: i16) {
        self.state.player1.waitroom.cards.push(id);
    }

    pub fn add_to_stage(&mut self, area: MemberArea, id: i16) {
        self.state.player1.stage.set_area(area, id);
    }

    pub fn give_energy(&mut self, count: usize) {
        self.give_energy_for(Side::P1, count);
    }

    /// ONE runner for every main-phase test action: optional trace label,
    /// engine call (ability-indexed or plain), state dump. All the
    /// `play_*/activate_*/set_live_card/pass` wrappers below delegate here
    /// so the trace/dump discipline can't diverge between them.
    fn run_main_action(
        &mut self,
        trace_label: Option<String>,
        action: &ActionType,
        card_id: Option<i16>,
        area: Option<MemberArea>,
        ability_index: Option<usize>,
    ) -> Result<(), String> {
        if let Some(label) = trace_label {
            if self.trace.live() {
                self.trace.emit(label);
            }
        }
        let r = if let Some(idx) = ability_index {
            TurnEngine::execute_main_phase_action_with_ability_index(
                &mut self.state,
                action,
                card_id,
                None,
                None,
                None,
                Some(idx),
            )
        } else {
            // Area-carrying (play-to-stage) calls pass Some(false) as the
            // trailing flag; all other actions pass None. Matches every
            // historical per-action call site.
            let flag = area.map(|_| false);
            TurnEngine::execute_main_phase_action(
                &mut self.state,
                action,
                card_id,
                None,
                area,
                flag,
            )
        };
        trace::dump_state(self, &self.trace);
        r
    }

    pub fn play_to_stage(&mut self, card_id: i16, area: MemberArea) {
        self.try_play_to_stage(card_id, area)
            .expect("play_to_stage failed");
    }

    pub fn try_play_to_stage(&mut self, card_id: i16, area: MemberArea) -> Result<(), String> {
        self.run_main_action(
            Some(format!(
                "A play {} {}",
                trace::ref_of(self, card_id),
                area.to_index()
            )),
            &ActionType::PlayMemberToStage,
            Some(card_id),
            Some(area),
            None,
        )
    }

    pub fn activate_ability(&mut self, stage_card_id: i16) {
        self.try_activate_ability(stage_card_id)
            .expect("activate_ability failed");
    }

    pub fn try_activate_ability(&mut self, stage_card_id: i16) -> Result<(), String> {
        self.run_main_action(
            Some(format!(
                "A activate {}",
                trace::ref_of(self, stage_card_id)
            )),
            &ActionType::UseAbility,
            Some(stage_card_id),
            None,
            None,
        )
    }

    pub fn activate_ability_index(&mut self, stage_card_id: i16, ability_index: usize) {
        self.try_activate_ability_index(stage_card_id, ability_index)
            .expect("activate_ability_index failed");
    }

    pub fn try_activate_ability_index(
        &mut self,
        stage_card_id: i16,
        ability_index: usize,
    ) -> Result<(), String> {
        self.run_main_action(
            Some(format!(
                "A activate_idx {} {}",
                trace::ref_of(self, stage_card_id),
                ability_index
            )),
            &ActionType::UseAbility,
            Some(stage_card_id),
            None,
            Some(ability_index),
        )
    }

    pub fn set_live_card(&mut self, card_id: i16) {
        self.run_main_action(
            Some(format!("A setlive {}", trace::ref_of(self, card_id))),
            &ActionType::SetLiveCard,
            Some(card_id),
            None,
            None,
        )
        .expect("set_live_card failed");
    }

    pub fn pass(&mut self) {
        self.run_main_action(Some("A pass".to_string()), &ActionType::Pass, None, None, None)
            .expect("pass failed");
    }

    /// Step the turn until `target` is the current phase.
    ///
    /// Prefer this over `for _ in 0..3 { game.pass() }`. A blind count works
    /// only while the phase sequence is frozen: the moment a phase gains or
    /// loses a step, the test is standing in a different window than it thinks,
    /// and the first thing that "breaks" is the test's own setup rather than the
    /// engine. Panics instead of spinning when the phase is never reached.
    ///
    /// Targets worth knowing (each established by driving a real live):
    ///   * `LiveCardSetFirstAttacker` / `LiveCardSetSecondAttacker` — where
    ///     `set_live_card` belongs, for the first and second attacker.
    ///   * `FirstAttackerPerformance` — where ライブ開始時 is scanned.
    ///   * `SecondAttackerPerformance` — where the YELL happens; the yell prompt
    ///     does not exist yet in the first attacker's phase, so a test waiting for
    ///     a SelectAutoAbility from a yell must step to this one or later.
    ///   * `LiveVictoryDetermination` — after both performances, with the
    ///     snapshots recorded.
    ///   * `Active` — one full turn later; the reliable way to be sure EVERY
    ///     live phase was traversed when the thing under test is ライブ成功時,
    ///     which resolves as the live closes. `LiveVictoryDetermination` is too
    ///     early for it.
    pub fn advance_to_phase(&mut self, target: rabuka_engine::game_state::Phase) {
        for _ in 0..16 {
            if self.state.current_phase == target {
                return;
            }
            self.pass();
            if self.state.current_phase == target {
                // Arrived: hand any prompt raised by this step to the CALLER.
                // The phase you are stepping into is often the one whose window
                // raises the prompt (a ライブ開始時 position change, say), and
                // draining it here would consume the very choice the test came
                // to inspect. An empty answer is not an option either — a
                // non-skippable prompt rejects it.
                return;
            }
            while self.has_pending_choice() {
                self.select_indices(&[0]);
            }
        }
        panic!(
            "the turn never reached {:?} within 16 passes (stuck at {:?})",
            target, self.state.current_phase
        );
    }
}

pub fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

pub fn put_on_deck_top(game: &mut TestGame, player: u8, card: i16) {
    game.state.seat_player_mut(player).main_deck.cards.insert(0, card);
}

pub fn fill_energy_deck(game: &mut TestGame, player: u8, count: usize) {
    let db = game.db.clone();
    let deck = &mut game.state.seat_player_mut(player).energy_deck;
    let energy = card_id(&db, "LL-E-001-SD");
    for _ in 0..count {
        deck.cards.push(energy);
    }
}
