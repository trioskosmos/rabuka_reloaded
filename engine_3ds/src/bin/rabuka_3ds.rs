// Rabuka 3DS — step-by-step loading + auto-play across frames.
//
// DESIGN NOTES (3DS pitfalls):
//
// 1. APT main loop: ALL game processing must happen inside the
//    `while aptMainLoop() { }` frame loop. If you block for more than
//    ~1 frame without calling aptMainLoop, the OS kills the app.
//    This is why loading is split into states, one per frame.
//
// 2. RomFS: cards.json and deck files are bundled into the .3dsx via
//    Cargo.toml's `[package.metadata.cargo-3ds] romfs_dir = "romfs"`.
//    Access via `romfs:/` paths (requires romfsInit() in ctru_shim.c).
//
// 3. pthread_atfork: rand 0.8.6's ReseedingRng calls libc::pthread_atfork
//    during thread_rng() initialization. On the 3DS this returns ENOSYS
//    (code 88) and panics. We override it with a no-op via #[no_mangle]
//    and `--allow-multiple-definition` in build.rs / linker flags.
//
// 4. Shuffle: DeckBuilder::shuffle_main/shuffle_energy use thread_rng()
//    which crashes on 3DS (TLS not supported). Currently skipped — the
//    deck plays in original file order. Replace once a TLS-free RNG
//    approach is available (e.g. SmallRng from_entropy).
//
// 5. getrandom: Provided via ctru_shim.c using svcGetSystemTick + xorshift64.
//    This is sufficient for card shuffling and the rand crate's needs.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::deck_builder::DeckBuilder;
use rabuka_engine::deck_parser::DeckParser;
use rabuka_engine::game_setup;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn;

#[cfg(feature = "3ds")]
enum LoadState {
    ReadFile,
    ReadAbilitiesMap(String),
    ParseCards(String, Vec<u8>),
    AttachAbilities(Vec<rabuka_engine::card::Card>, Vec<u8>),
    BuildGame(Vec<rabuka_engine::card::Card>),
    Playing(GameState, usize),
    Done(Result<(), String>),
}

#[cfg(feature = "3ds")]
/// Override pthread_atfork to avoid panics in rand 0.8.6's ReseedingRng.
/// The 3DS doesn't support fork(), so this function is a no-op.
/// Rust's #[no_mangle] ensures this definition takes precedence over
/// the one in libsysbase (which returns ENOSYS, causing a panic).
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
    let mut state = LoadState::ReadFile;

    while unsafe { _3ds_main_loop() != 0 } {
        print!("\x1b[2J");
        println!("Rabuka 3DS");
        println!();

        match &state {
            LoadState::Done(result) => match result {
                Ok(_) => println!("Done! Press START to exit."),
                Err(e) => println!("ERROR: {}", e),
            },
            LoadState::Playing(gs, _) => {
                println!("Turn {} | Phase: {:?}", gs.turn_number, gs.current_phase);
                println!(
                    "P1 hand:{} energy:{} stage:{}/3",
                    gs.player1.hand.cards.len(),
                    gs.player1.energy_zone.cards.len(),
                    gs.player1.stage.stage.iter().filter(|&&s| s != -1).count()
                );
                println!(
                    "P2 hand:{} energy:{} stage:{}/3",
                    gs.player2.hand.cards.len(),
                    gs.player2.energy_zone.cards.len(),
                    gs.player2.stage.stage.iter().filter(|&&s| s != -1).count()
                );
            }
            _ => {}
        }

        state = match state {
            LoadState::ReadFile => {
                println!("[1/5] Reading cards.json...");
                let path = Path::new("romfs:/cards.json");
                match File::open(path).and_then(|mut f| {
                    let mut s = String::new();
                    f.read_to_string(&mut s).map(|_| s)
                }) {
                    Ok(c) => {
                        println!("  {} bytes read", c.len());
                        LoadState::ReadAbilitiesMap(c)
                    }
                    Err(e) => LoadState::Done(Err(format!("Read cards: {}", e))),
                }
            }
            LoadState::ReadAbilitiesMap(cards_json) => {
                // Load the pre-baked compact abilities map (card_no -> Vec<Ability>).
                // Generated at build time by gen_abilities_map desktop tool.
                // Uses bincode for instantaneous loading on the 3DS ARM11.
                println!("[2/5] Reading abilities_map.bin...");
                let path = Path::new("romfs:/abilities_map.bin");
                match File::open(path).and_then(|mut f| {
                    let mut s = Vec::new();
                    f.read_to_end(&mut s).map(|_| s)
                }) {
                    Ok(c) => {
                        println!("  {} bytes read", c.len());
                        LoadState::ParseCards(cards_json, c)
                    }
                    Err(_) => {
                        println!("  No abilities_map.bin, proceeding without.");
                        LoadState::ParseCards(cards_json, Vec::new())
                    }
                }
            }
            LoadState::ParseCards(cards_json, abilities_map_json) => {
                println!("[3/5] Parsing cards...");
                unsafe { _3ds_swap_buffers(); }
                match CardLoader::load_cards_from_strs(&cards_json, None) {
                    Ok(cards) => {
                        println!("  {} cards parsed", cards.len());
                        if abilities_map_json.is_empty() {
                            LoadState::BuildGame(cards)
                        } else {
                            // cards_json dropped here — frees ~3MB before ability map parse
                            LoadState::AttachAbilities(cards, abilities_map_json)
                        }
                    }
                    Err(e) => LoadState::Done(Err(format!("Parse cards: {}", e))),
                }
            }
            LoadState::AttachAbilities(cards, abilities_map_bin) => {
                println!("[4/5] Attaching abilities...");
                unsafe { _3ds_swap_buffers(); }
                // Deserialize directly from bincode bytes.
                #[derive(serde::Deserialize)]
                struct AbilitiesMapFile {
                    abilities: Vec<rabuka_engine::card::Ability>,
                    cards: HashMap<String, Vec<usize>>,
                }
                match rmp_serde::from_slice::<AbilitiesMapFile>(&abilities_map_bin) {
                    Ok(map_file) => {
                        let cards = CardLoader::apply_abilities_index(
                            cards,
                            &map_file.abilities,
                            &map_file.cards,
                        );
                        println!("  {} cards with abilities", map_file.cards.len());
                        LoadState::BuildGame(cards)
                    }
                    Err(e) => {
                        println!("  abilities map error: {}", e);
                        LoadState::BuildGame(cards)
                    }
                }
            }
            LoadState::BuildGame(cards) => {
                println!("[5/6] Building game state...");
                println!("  Creating database...");
                let db = Arc::new(CardDatabase::load_or_create(cards));
                println!("  Loading decks...");
                match DeckParser::parse_all_decks_from_directory(Path::new("romfs:/decks/")) {
                    Ok(decks) if !decks.is_empty() => {
                        let d = decks[0].clone();
                        let nums = DeckParser::deck_list_to_card_numbers(&d);
                        println!("  Building deck...");
                        match DeckBuilder::build_deck_from_database(&mut db.clone(), nums) {
                            Ok(mut pd) => {
                                // Skip shuffle on 3DS: thread_rng() uses TLS which
                                // isn't supported on this target and causes a crash.
                                // The deck is still playable in original file order.
                                println!("  Building deck...");
                                DeckBuilder::add_default_energy_cards_from_database(
                                    &mut pd,
                                    &mut db.clone(),
                                )
                                .ok();

                                println!("  Creating players...");
                                let mut p1 = Player::new("p1".into(), "P1".into(), true);
                                p1.set_main_deck(pd.main_deck.clone());
                                p1.set_energy_deck(pd.energy_deck.clone());
                                let mut p2 = Player::new("p2".into(), "P2".into(), false);
                                p2.set_main_deck(pd.main_deck);
                                p2.set_energy_deck(pd.energy_deck);

                                println!("  Initializing game...");
                                let mut gs = GameState::new(p1, p2, db);
                                game_setup::setup_game(&mut gs);
                                println!("  Game ready!");
                                LoadState::Playing(gs, 0)
                            }
                            Err(e) => LoadState::Done(Err(format!("Build deck: {}", e))),
                        }
                    }
                    Ok(_) => LoadState::Done(Err("No decks found".into())),
                    Err(e) => LoadState::Done(Err(format!("Decks: {}", e))),
                }
            }
            LoadState::Playing(mut gs, mut cursor) => {
                if gs.game_result != rabuka_engine::game_state::GameResult::Ongoing {
                    println!("Game ended: {:?}", gs.game_result);
                    LoadState::Done(Ok(()))
                } else {
                    let actions = game_setup::generate_possible_actions(&gs);
                    if actions.is_empty() {
                        LoadState::Done(Err("No actions".into()))
                    } else {
                        if cursor >= actions.len() {
                            cursor = actions.len() - 1;
                        }
                        
                        println!("  {} actions available", actions.len());
                        let start_idx = if cursor >= 10 { cursor - 10 } else { 0 };
                        for (i, action) in actions.iter().enumerate().skip(start_idx).take(15) {
                            if i == cursor {
                                println!("> [{}] {}", i, action.description);
                            } else {
                                println!("  [{}] {}", i, action.description);
                            }
                        }

                        unsafe { _3ds_scan_input(); }
                        let keys = unsafe { _3ds_keys_down() };
                        const KEY_A: u32 = 1 << 0;
                        const KEY_DPAD_UP: u32 = 1 << 6;
                        const KEY_DPAD_DOWN: u32 = 1 << 7;

                        if (keys & KEY_DPAD_UP) != 0 && cursor > 0 {
                            cursor -= 1;
                        } else if (keys & KEY_DPAD_DOWN) != 0 && cursor + 1 < actions.len() {
                            cursor += 1;
                        } else if (keys & KEY_A) != 0 {
                            execute_action(&mut gs, &actions, cursor);
                            cursor = 0;
                        }
                        LoadState::Playing(gs, cursor)
                    }
                }
            }
            LoadState::Done(_) => state,
        };

        unsafe {
            _3ds_swap_buffers();
        }
    }
    unsafe {
        _3ds_exit();
    }
}

// pick_action is removed in interactive mode

#[cfg(feature = "3ds")]
fn execute_action(gs: &mut GameState, actions: &[game_setup::Action], idx: usize) {
    if idx >= actions.len() {
        return;
    }
    let a = &actions[idx];
    let p = a.parameters.clone();

    if let Err(e) = rabuka_engine::turn::TurnEngine::execute_main_phase_action(
        gs,
        &a.action_type,
        p.as_ref().and_then(|p| p.card_id),
        p.as_ref().and_then(|p| p.card_indices.clone()),
        p.as_ref()
            .and_then(|p| p.stage_area.as_ref().and_then(|s| s.parse().ok())),
        p.as_ref().and_then(|p| p.use_baton_touch),
    ) {
        println!("Action error: {}", e);
        return;
    }

    gs.reset_loop_detection();

    loop {
        if gs.has_pending_choice() {
            break;
        }
        if gs.game_result != rabuka_engine::game_state::GameResult::Ongoing {
            break;
        }
        let auto = matches!(
            gs.current_phase,
            rabuka_engine::game_state::Phase::Active
                | rabuka_engine::game_state::Phase::Energy
                | rabuka_engine::game_state::Phase::Draw
                | rabuka_engine::game_state::Phase::FirstAttackerPerformance
                | rabuka_engine::game_state::Phase::SecondAttackerPerformance
                | rabuka_engine::game_state::Phase::LiveVictoryDetermination
        );
        if !auto {
            break;
        }
        turn::TurnEngine::advance_phase(gs);
    }
}

extern "C" {
    fn _3ds_init();
    fn _3ds_main_loop() -> i32;
    fn _3ds_exit();
    fn _3ds_swap_buffers();
    fn _3ds_scan_input();
    fn _3ds_keys_down() -> u32;
}

#[cfg(not(feature = "3ds"))]
fn main() {
    println!("Desktop mode - use: cargo run --bin harness");
}
