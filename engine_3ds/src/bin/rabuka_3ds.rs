// Rabuka 3DS — interactive card game.  All work inside APT main loop.
//
// RAM constraints (measured with --release on desktop, 3DS ARM11 ~10x slower):
//   sizeof(Ability) = 19968 bytes  // 20 KB each — dozens of Option<Box<...>> fields
//   sizeof(Card) = 504 bytes
//   2280 cards → ~1.1 MB
//   cards.bin: 2100 KB (MessagePack, 33% smaller than JSON)
//   abilities.json: 1453 KB on disk
//
// Loading strategy (abilities deferred to after deck selection):
//   1) Read cards.bin via rmp_serde + YieldReader → Vec<Card> (no abilities, ~2s)
//   2) Select two player decks → ~120 unique card_nos
//   3) Read abilities.json, build ability map, attach ONLY for deck cards
//      (~120 clones × 20KB = ~3MB instead of 33MB for all 1727)
//   4) Build game and play
// This avoids the 3DS watchdog (no extended JSON parsing) and saves ~30MB RAM.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_builder::{Deck, DeckBuilder};
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use rabuka_engine::turn;

/// 3DS system tick rate: 268.12 MHz (ARM11)
const TICK_HZ: u64 = 268_120_000;
/// Print debug timing every N frames (0 = disabled)
const DBG_EVERY_N: u64 = 60;

#[cfg(feature = "3ds")]
struct AptReader<R> {
    inner: R,
    threshold: usize,
    counter: usize,
}

#[cfg(feature = "3ds")]
impl<R: Read> Read for AptReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.counter += n;
        if self.counter >= self.threshold {
            self.counter = 0;
            let _ = unsafe { _3ds_keep_alive() };
        }
        Ok(n)
    }
}

/// Reader wrapper that calls aptMainLoop() every `threshold` bytes without
/// any GPU buffer operations. Keeps the 3DS OS alive during long deserialization
/// without the overhead/cost of _3ds_keep_alive().
#[cfg(feature = "3ds")]
struct YieldReader<R> {
    inner: R,
    threshold: usize,
    counter: usize,
}

#[cfg(feature = "3ds")]
impl<R: Read> Read for YieldReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.counter += n;
        if self.counter >= self.threshold {
            self.counter = 0;
            if unsafe { _3ds_main_loop() } == 0 {
                // App should exit; return empty read to signal EOF
                return Ok(0);
            }
        }
        Ok(n)
    }
}

// dprintln! — game output on BOTTOM screen (user choices).
// Also sends to debug console via 3dslink.
#[cfg(feature = "3ds")]
macro_rules! dprintln {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\n\0", msg);
        unsafe { _3ds_debug_print(s.as_ptr()); }
        // Default console is BOTTOM — println! goes there
        println!("{}", msg);
    }};
}

// tprintln! — debug output on TOP screen (timing/status).
// Switches to top console temporarily, then back to bottom.
#[cfg(feature = "3ds")]
macro_rules! tprintln {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let s = format!("{}\n\0", msg);
        unsafe {
            _3ds_debug_print(s.as_ptr());
            _3ds_select_top();
        }
        println!("{}", msg);
        unsafe { _3ds_select_bottom(); }
    }};
}

#[cfg(feature = "3ds")]
fn ticks_to_ms(ticks: u64) -> f64 {
    (ticks as f64) / (TICK_HZ as f64) * 1000.0
}

#[cfg(feature = "3ds")]
enum Step {
    ReadCardsBin,
    ParseCards(Vec<u8>),
    LoadDecks(Vec<Card>),
    Play(GameState, usize, Vec<game_setup::Action>, bool, bool),
    Done(Result<(), String>),
}

#[cfg(feature = "3ds")]
#[no_mangle]
pub unsafe extern "C" fn pthread_atfork(
    _prepare: Option<unsafe extern "C" fn()>,
    _parent: Option<unsafe extern "C" fn()>,
    _child: Option<unsafe extern "C" fn()>,
) -> i32 {
    0
}

#[cfg(feature = "3ds")]
fn main() {
    std::panic::set_hook(Box::new(|info| {
        print!("\x1b[2JPANIC!\n");
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            print!("  {}\n", s);
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            print!("  {}\n", s);
        } else {
            print!("  (no message)\n");
        }
        if let Some(loc) = info.location() {
            print!("  at {}:{}\n", loc.file(), loc.line());
        }
        loop {
            unsafe {
                _3ds_swap_buffers();
            }
        }
    }));
    unsafe {
        _3ds_init();
    }

    let mut frame: u64 = 0;
    let mut step = Step::ReadCardsBin;

    while unsafe { _3ds_main_loop() != 0 } {
        let tick_start = unsafe { _3ds_system_tick() };

        unsafe {
            _3ds_scan_input();
        }
        let keys = unsafe { _3ds_keys_down() };
        if keys & 0x00000008 != 0 {
            break;
        }

        if frame >= DBG_EVERY_N && frame % DBG_EVERY_N == 0 {
            tprintln!("[DBG] frame={} step={}", frame, step_name(&step));
        }
        let current_step = step_name(&step);
        frame += 1;

        step = match step {
            Step::ReadCardsBin => {
                let t0 = unsafe { _3ds_system_tick() };
                println!("[1/2] Reading cards.bin...");
                let path = Path::new("romfs:/cards.bin");
                match File::open(path).and_then(|mut f| {
                    let mut v = Vec::new();
                    f.read_to_end(&mut v).map(|_| v)
                }) {
                    Ok(v) => {
                        let t1 = unsafe { _3ds_system_tick() };
                        dprintln!("  {} B ({} ms)", v.len(), ticks_to_ms(t1 - t0));
                        Step::ParseCards(v)
                    }
                    Err(e) => Step::Done(Err(format!("Read: {}", e))),
                }
            }
            Step::ParseCards(bytes) => {
                let t0 = unsafe { _3ds_system_tick() };
                println!("[2/3] Deserializing cards...");
                let reader = YieldReader {
                    inner: std::io::Cursor::new(&bytes),
                    threshold: 8192,
                    counter: 0,
                };
                match rmp_serde::from_read::<_, HashMap<String, Card>>(reader) {
                    Ok(map) => {
                        let t1 = unsafe { _3ds_system_tick() };
                        let cards: Vec<_> = map.into_values().collect();
                        dprintln!("  {} cards ({} ms)", cards.len(), ticks_to_ms(t1 - t0));
                        drop(bytes);
                        Step::LoadDecks(cards)
                    }
                    Err(e) => Step::Done(Err(format!("Parse: {}", e))),
                }
            }
            Step::LoadDecks(cards) => {
                let t0 = unsafe { _3ds_system_tick() };
                println!("[3/3] Building game...");

                let mut db = Arc::new(CardDatabase::load_or_create(cards));

                // Build decks
                let decks =
                    match DeckParser::parse_all_decks_from_directory(Path::new("romfs:/decks/")) {
                        Ok(v) if !v.is_empty() => v,
                        _ => {
                            step = Step::Done(Err("No decks".into()));
                            continue;
                        }
                    };
                let nums = DeckParser::deck_list_to_card_numbers(&decks[0]);
                let (mut pd1, mut pd2) = match (
                    DeckBuilder::build_deck_from_database(&mut db, nums.clone()),
                    DeckBuilder::build_deck_from_database(&mut db, nums),
                ) {
                    (Ok(pd1), Ok(pd2)) => (pd1, pd2),
                    _ => {
                        step = Step::Done(Err("Failed to build decks".into()));
                        continue;
                    }
                };
                pd1.shuffle_main_deck();
                pd1.shuffle_energy_deck();
                pd2.shuffle_main_deck();
                pd2.shuffle_energy_deck();
                let decks_t = unsafe { _3ds_system_tick() };
                dprintln!("  Decks built ({} ms)", ticks_to_ms(decks_t - t0));

                // 3. Collect unique card_nos from both decks
                let mut deck_nos: HashSet<String> = HashSet::new();
                for cid in pd1
                    .main_deck
                    .iter()
                    .chain(pd1.energy_deck.iter())
                    .chain(pd2.main_deck.iter())
                    .chain(pd2.energy_deck.iter())
                {
                    if let Some(card) = db.get_card(*cid) {
                        deck_nos.insert(card.card_no.clone());
                    }
                }

                // 4. Attach abilities ONLY for deck cards
                let ab_path = Path::new("romfs:/abilities.json");
                match File::open(ab_path).and_then(|mut f| {
                    let mut v = String::new();
                    f.read_to_string(&mut v).map(|_| v)
                }) {
                    Ok(json) => {
                        let attach_t0 = unsafe { _3ds_system_tick() };
                        if let Ok(abilities_data) = CardLoader::load_abilities_from_str(&json) {
                            let ability_map = CardLoader::build_abilities_map(&abilities_data);
                            drop(abilities_data);
                            let db_inner = Arc::make_mut(&mut db);
                            for (_, card) in db_inner.cards.iter_mut() {
                                if deck_nos.contains(&card.card_no) {
                                    if let Some(ab) = ability_map.get(&card.card_no) {
                                        card.abilities = ab.clone();
                                    }
                                }
                            }
                            let attach_t1 = unsafe { _3ds_system_tick() };
                            dprintln!(
                                "  Abilities attached ({} ms, {} deck cards)",
                                ticks_to_ms(attach_t1 - attach_t0),
                                deck_nos.len()
                            );
                        } else {
                            dprintln!("  abilities.json parse failed");
                        }
                    }
                    Err(e) => dprintln!("  abilities.json read failed: {}", e),
                }

                // 5. Add energy cards and build players
                DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
                DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();

                let mut p1 = Player::new("p1".into(), "P1".into(), true);
                p1.set_main_deck(pd1.main_deck);
                p1.set_energy_deck(pd1.energy_deck);
                let mut p2 = Player::new("p2".into(), "P2".into(), false);
                p2.set_main_deck(pd2.main_deck);
                p2.set_energy_deck(pd2.energy_deck);

                let mut gs = GameState::new(p1, p2, db);
                game_setup::setup_game(&mut gs);
                let t1 = unsafe { _3ds_system_tick() };
                dprintln!("  Game ready ({} ms)", ticks_to_ms(t1 - t0));
                Step::Play(gs, 0, Vec::new(), true, true)
            }
            Step::Play(mut gs, mut cur, mut acts_cache, mut dirty, mut redraw) => {
                // Handle cursor movement via D-pad
                let n = acts_cache.len();
                if keys & 0x00000040 != 0 && cur > 0 {
                    cur -= 1;
                    redraw = true;
                } else if keys & 0x00000080 != 0 && cur + 1 < n {
                    cur += 1;
                    redraw = true;
                }

                // Handle action selection via A button
                if keys & 0x00000001 != 0 && cur < n {
                    let action = acts_cache[cur].clone();
                    let p = action.parameters.clone();
                    tprintln!(
                        "[ACT] {:?} phase={:?}",
                        action.action_type,
                        gs.current_phase
                    );
                    let t_exec = unsafe { _3ds_system_tick() };
                    let result = turn::TurnEngine::execute_main_phase_action(
                        &mut gs,
                        &action.action_type,
                        p.as_ref().and_then(|x| x.card_id),
                        p.as_ref().and_then(|x| x.card_indices.clone()),
                        p.as_ref()
                            .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
                        p.as_ref().and_then(|x| x.use_baton_touch),
                    );
                    let t_exec_end = unsafe { _3ds_system_tick() };
                    let exec_ms = ticks_to_ms(t_exec_end - t_exec);
                    tprintln!(
                        "[ACT] done in {} ms -> phase={:?} err={}",
                        exec_ms,
                        gs.current_phase,
                        if let Err(ref e) = result {
                            e.as_str()
                        } else {
                            "ok"
                        }
                    );
                    gs.reset_loop_detection();
                    gs.reset_loop_detection();
                    cur = 0;
                    dirty = true;
                    redraw = true;
                }

                // Recalculate bounds after action may have changed act count
                let n2 = acts_cache.len();
                if n2 > 0 && cur >= n2 {
                    cur = n2 - 1;
                }

                // --- AUTO-ADVANCE ---
                // Mirror the web server exactly: run settle_single_player_state which loops
                // through Active→Energy→Draw→Main in one synchronous call, then stops at the
                // first phase requiring user input or a pending ability choice.
                let auto = !gs.has_pending_choice()
                    && gs.game_result == GameResult::Ongoing
                    && game_setup::is_automatic_phase(&gs);
                tprintln!(
                    "[LOOP] phase={:?} turn={} pend={} auto={}",
                    gs.current_phase,
                    gs.turn_number,
                    gs.has_pending_choice(),
                    auto
                );
                if auto {
                    tprintln!("[SETTLE] start {:?}", gs.current_phase);
                    settle_3ds(&mut gs);
                    tprintln!("[SETTLE] end -> {:?}", gs.current_phase);
                    dirty = true;
                }

                // Redraw on state change or cursor move
                if dirty || redraw {
                    let t_gen = unsafe { _3ds_system_tick() };
                    acts_cache = game_setup::generate_possible_actions(&gs);
                    let t_gen_end = unsafe { _3ds_system_tick() };
                    let gen_ms = ticks_to_ms(t_gen_end - t_gen);
                    if gen_ms > 100.0 {
                        tprintln!("[WARN] generate took {} ms", gen_ms);
                    }
                    unsafe {
                        _3ds_clear_console();
                    }
                    dprintln!(
                        "phase={:?} turn={} result={:?}  [A]=select",
                        gs.current_phase,
                        gs.turn_number,
                        gs.game_result,
                    );
                    // Board state header
                    let ap = gs.active_player();
                    let card_name = |cid| {
                        gs.card_database
                            .get_card(cid)
                            .map(|c| c.card_no.as_str())
                            .unwrap_or("??")
                    };
                    let stage_cids = [ap.stage.stage[0], ap.stage.stage[1], ap.stage.stage[2]];
                    let stage_str = if (0..3).any(|i| stage_cids[i] != -1) {
                        let parts: Vec<String> = (0..3)
                            .map(|i| {
                                if stage_cids[i] == -1 {
                                    " - ".into()
                                } else {
                                    format!("{}", card_name(stage_cids[i]))
                                }
                            })
                            .collect();
                        format!("St:L{} C{} R{}", parts[0], parts[1], parts[2])
                    } else {
                        "St: empty".into()
                    };
                    let hand_n = ap.hand.cards.len();
                    let energy_n = ap.energy_zone.active_count();
                    let live_n = ap.success_live_card_zone.cards.len();
                    dprintln!("{} | H:{} E:{} L:{}", stage_str, hand_n, energy_n, live_n);
                    let n = acts_cache.len();
                    if n > 0 && cur >= n {
                        cur = n - 1;
                    }
                    let window_size = 20;
                    let start_idx = if n > window_size {
                        cur.saturating_sub(window_size / 2).min(n - window_size)
                    } else {
                        0
                    };
                    let end_idx = (start_idx + window_size).min(n);

                    if start_idx > 0 {
                        dprintln!("... ({} more above)", start_idx);
                    }

                    for (i, a) in acts_cache
                        .iter()
                        .enumerate()
                        .skip(start_idx)
                        .take(end_idx - start_idx)
                    {
                        let arrow = if i == cur { ">" } else { " " };
                        let mut desc = a.description.clone();
                        if let Some(p) = &a.parameters {
                            if let Some(card_id) = p.card_id {
                                if let Some(card) = gs.card_database.get_card(card_id) {
                                    desc = format!("[{}] {}", card.card_no, desc);
                                }
                            }
                        }
                        let safe_desc: String = desc
                            .chars()
                            .map(|c| if c.is_ascii() { c } else { '?' })
                            .collect();
                        dprintln!("{} [{}] {:?} {}", arrow, i, a.action_type, safe_desc);
                    }

                    if end_idx < n {
                        dprintln!("... ({} more below)", n - end_idx);
                    }

                    if gs.game_result != GameResult::Ongoing {
                        dprintln!("Game ended: {:?}", gs.game_result);
                    }
                    dirty = false;
                    redraw = false;
                }
                Step::Play(gs, cur, acts_cache, dirty, redraw)
            }
            Step::Done(ref r) => {
                print!("\x1b[2J");
                match r {
                    Ok(_) => println!("Done! Press START."),
                    Err(e) => println!("ERROR: {}", e),
                }
                if keys & 0x00000008 != 0 {
                    break;
                }
                Step::Done(match r {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.clone()),
                })
            }
        };
        let tick_end = unsafe { _3ds_system_tick() };
        let frame_ms = ticks_to_ms(tick_end - tick_start);
        if frame_ms > 33.0 {
            tprintln!(
                "[WARN] frame {}: {} ms (step: {})",
                frame,
                frame_ms,
                current_step
            );
        }
        unsafe {
            _3ds_swap_buffers();
        }
    }
    unsafe {
        _3ds_exit();
    }
}

/// 3DS-native settle: same logic as game_setup::settle_single_player_state but
/// calls aptMainLoop() every 10 iterations to keep the OS watchdog happy, and
/// avoids ALL eprintln!/log calls (which can deadlock the GPU console renderer).
#[cfg(feature = "3ds")]
fn settle_3ds(gs: &mut GameState) {
    let mut iters = 0u32;
    loop {
        iters += 1;
        // Yield to OS every 10 iterations to avoid watchdog timeout
        if iters % 10 == 0 {
            if unsafe { _3ds_main_loop() } == 0 {
                return;
            }
        }
        if iters > 500 {
            break;
        }
        if gs.has_pending_choice() {
            break;
        }
        if gs.game_result != GameResult::Ongoing {
            break;
        }
        if game_setup::is_automatic_phase(gs) {
            turn::TurnEngine::advance_phase(gs);
        } else {
            break;
        }
    }
}

#[cfg(feature = "3ds")]
fn step_name(s: &Step) -> &'static str {
    match s {
        Step::ReadCardsBin => "ReadCards",
        Step::ParseCards(_) => "ParseCards",
        Step::LoadDecks(_) => "LoadDecks",
        Step::Play(_, _, _, _, _) => "Play",
        Step::Done(_) => "Done",
    }
}

extern "C" {
    fn _3ds_init();
    fn _3ds_main_loop() -> i32;
    fn _3ds_keep_alive() -> i32;
    fn _3ds_exit();
    fn _3ds_swap_buffers();
    fn _3ds_scan_input();
    fn _3ds_keys_down() -> u32;
    fn _3ds_system_tick() -> u64;
    fn _3ds_debug_print(msg: *const u8);
    fn _3ds_select_top();
    fn _3ds_select_bottom();
    fn _3ds_clear_console();
}

#[cfg(not(feature = "3ds"))]
fn main() {
    println!("Desktop mode - use: cargo run --bin harness");
}
