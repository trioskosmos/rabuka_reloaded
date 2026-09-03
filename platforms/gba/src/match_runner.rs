//! GBA-specific match runner with audio tracker support.
//!
//! Uses agb-tracker crate for .xm tracker music (much smaller than WAV).
//! Samples + patterns take only KBs vs MBs for raw PCM audio.

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::card::Card;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::compat::psp_hash::HashSet;
use rabuka_engine::game::deck_builder::DeckBuilder;
use rabuka_engine::game::game_setup;
use rabuka_engine::game::menu::{handle_choice, human_turn, show_result};
use rabuka_engine::game::platform_ui::{MatchMode, PlatformUi};
use rabuka_engine::game_state::{GameResult, GameState, Phase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;

use agb::sound::mixer::{Mixer, Frequency};
use agb_tracker::{Track, Tracker, include_xm};

/// Background music as tracker module (.xm format - samples + patterns).
/// Compile-time loaded via include_xm! macro.
static BGM_TRACK: Track = include_xm!("sfx/next_card.xm");

/// AI turn: pick a random action and execute it.
fn ai_turn(gs: &mut GameState, acts: &[game_setup::Action]) -> bool {
    use rabuka_engine::game_setup::ActionType;
    for a in acts {
        if matches!(
            a.action_type,
            ActionType::ConfirmMulligan | ActionType::SkipMulligan
        ) {
            let _ = game_setup::execute_action(gs, a);
            return true;
        }
    }
    let _ = game_setup::execute_action(gs, &acts[rabuka_engine::rng::rand_range(acts.len())]);
    true
}

/// Auto-resolve a pending choice for the AI (random but legal).
fn ai_handle_choice(gs: &mut GameState) -> bool {
    use rabuka_engine::ability::types::Choice;
    let choice = match gs.get_pending_choice() {
        Some(c) => c.clone(),
        None => return true,
    };
    match &choice {
        Choice::SelectAutoAbility { options, .. } => {
            if options.is_empty() {
                return TurnEngine::resume_with_choice(gs, Some(0), None).is_ok();
            }
            let idx = rabuka_engine::rng::rand_range(options.len()) as i16;
            TurnEngine::resume_with_choice(gs, Some(idx), None).is_ok()
        }
        Choice::SelectLiveSuccess { options, .. } => {
            if options.is_empty() {
                return TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).is_ok();
            }
            let idx = rabuka_engine::rng::rand_range(options.len());
            TurnEngine::resume_with_choice(gs, None, Some(vec![idx])).is_ok()
        }
        Choice::SelectHeartColor { options, .. }
        | Choice::SelectHeartType { options, .. } => {
            if options.is_empty() {
                return TurnEngine::resume_with_choice(gs, Some(0), None).is_ok();
            }
            let idx = rabuka_engine::rng::rand_range(options.len()) as i16;
            TurnEngine::resume_with_choice(gs, Some(idx), None).is_ok()
        }
        Choice::SelectTarget { options, target, .. } => {
            let n = options.as_ref().map(|o| o.len()).unwrap_or(2).max(1);
            let idx = (rabuka_engine::rng::rand_range(n) as i16).min(n as i16 - 1);
            match target.as_str() {
                "choice" | "choice_string" | "conditional_optional" => {
                    TurnEngine::resume_with_choice(gs, None, Some(vec![idx as usize])).is_ok()
                }
                _ => TurnEngine::resume_with_choice(gs, Some(idx), None).is_ok(),
            }
        }
        Choice::SelectPosition { .. } => {
            let idx = rabuka_engine::rng::rand_range(3) as i16;
            TurnEngine::resume_with_choice(gs, Some(idx), None).is_ok()
        }
        Choice::SelectCard {
            zone,
            count,
            allow_skip,
            target_player_id,
            filtered_indices,
            ..
        } => {
            let player = target_player_id
                .as_ref()
                .and_then(|pid| {
                    if pid == &gs.player1.id {
                        Some(&gs.player1)
                    } else if pid == &gs.player2.id {
                        Some(&gs.player2)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| gs.active_player());
            let card_ids = rabuka_engine::ability::util::zone_cards(player, zone);
            let n_options = filtered_indices
                .as_ref()
                .map(|fi| fi.len())
                .unwrap_or(card_ids.len());
            if n_options == 0 {
                return TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).is_ok();
            }
            let _ = allow_skip;
            if *count <= 1 {
                let idx = rabuka_engine::rng::rand_range(n_options);
                let actual = filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx);
                TurnEngine::resume_with_choice(gs, None, Some(vec![actual])).is_ok()
            } else {
                let want = (*count).min(n_options);
                let mut picked = Vec::with_capacity(want);
                let mut seen: HashSet<usize> = HashSet::default();
                while picked.len() < want {
                    let idx = rabuka_engine::rng::rand_range(n_options);
                    if seen.insert(idx) {
                        picked.push(filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx));
                    }
                    if seen.len() == n_options {
                        break;
                    }
                }
                TurnEngine::resume_with_choice(gs, None, Some(picked)).is_ok()
            }
        }
    }
}

/// Run a full embedded match with per-frame tracker updates (for GBA audio).
pub fn run_match_with_mixer<U: PlatformUi>(
    ui: &mut U,
    p1_cards: &[&str],
    p2_cards: &[&str],
    all_cards: Vec<Card>,
    mode: MatchMode,
    mixer: &mut Mixer,
    vblank: &agb::interrupt::VBlank,
) -> GameResult {
    // Initialize tracker for background music
    let mut bgm_tracker = Tracker::new(&BGM_TRACK);

    let mut db = Rc::new(CardDatabase::load_or_create(all_cards));

    let nums1: Vec<String> = p1_cards.iter().map(|c| c.to_string()).collect();
    let nums2: Vec<String> = p2_cards.iter().map(|c| c.to_string()).collect();

    let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums1).expect("build P1 deck");
    let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums2).expect("build P2 deck");
    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    pd1.shuffle_main_deck();
    pd1.shuffle_energy_deck();
    pd2.shuffle_main_deck();
    pd2.shuffle_energy_deck();

    let mut p1 = Player::new("p1".into(), "Player 1".into(), true);
    p1.set_main_deck(pd1.main_deck);
    p1.set_energy_deck(pd1.energy_deck);
    let mut p2 = Player::new("p2".into(), "Player 2".into(), false);
    p2.set_main_deck(pd2.main_deck);
    p2.set_energy_deck(pd2.energy_deck);

    let mut gs = GameState::new(p1, p2, db);
    game_setup::setup_game(&mut gs);

    loop {
        TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(ui, &gs);
            break;
        }
        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(ui, &gs);
            break;
        }
        if gs.is_loop_detected() {
            show_result(ui, &gs);
            break;
        }

        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            TurnEngine::advance_phase(&mut gs);
            gs.reset_loop_detection();
            continue;
        }

        if gs.has_pending_choice() {
            let is_ai_choice = match mode {
                MatchMode::AiVsAi => true,
                MatchMode::VsAi => !gs.can_player_act(0),
                MatchMode::TwoPlayer => false,
            };
            if is_ai_choice {
                if !ai_handle_choice(&mut gs) {
                    break;
                }
            } else {
                if !handle_choice(ui, &mut gs) {
                    break;
                }
            }
            gs.reset_loop_detection();
            continue;
        }

        let rps_ai_turn = matches!(mode, MatchMode::VsAi)
            && gs.current_phase == Phase::RockPaperScissors
            && gs.player1_rps_choice.is_some();
        let is_ai = matches!(mode, MatchMode::AiVsAi)
            || rps_ai_turn
            || (matches!(mode, MatchMode::VsAi) && gs.active_player().id != gs.player1.id);
        let ok = if is_ai {
            ai_turn(&mut gs, &acts)
        } else {
            human_turn(ui, &mut gs, &acts)
        };
        if !ok {
            break;
        }
        gs.reset_loop_detection();
        game_setup::settle_auto(&mut gs);

        // Update tracker + mixer once per frame
        bgm_tracker.step(mixer);
        mixer.frame();
        vblank.wait_for_vblank();
    }

    gs.game_result
}