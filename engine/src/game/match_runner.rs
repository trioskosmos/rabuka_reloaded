//! Shared embedded match runner.
//!
//! This is the single implementation of the match front-end that the
//! per-console targets used to copy ~150 lines of: mode/table select, deck
//! build, player setup, and the "keep-the-match-honest" loop
//! (check_victory -> settle_auto -> loop-detect -> act). Every platform's
//! `main()` should call `run_embedded_game` and supply only its backend Ui and
//! a way to load the card set + its encoded decks. This mirrors the sequence
//! the web server uses (`game_setup::generate_possible_actions`,
//! `game_setup::settle_auto`, `TurnEngine::check_victory_condition`, ...) so
//! host and consoles converge on one loop contract.

#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
#[cfg(feature = "no_std")]
use alloc::vec::Vec;

use crate::card::Card;
use crate::card::CardDatabase;
use crate::game::deck_builder::DeckBuilder;
use crate::game::game_setup;
use crate::game::menu::{handle_choice, human_turn, select, show_result};
use crate::game::platform_ui::PlatformUi;
use crate::game_state::{GameResult, GameState, Phase};
use crate::player::Player;
use crate::rng;
use crate::turn::TurnEngine;
use crate::Arc;

/// Auto-resolve a pending choice for the AI (random but legal). Returns false
/// only if the engine rejects the synthetic answer.
fn ai_handle_choice(gs: &mut GameState) -> bool {
    use crate::ability::types::Choice;
    let choice = match gs.get_pending_choice() {
        Some(c) => c.clone(),
        None => return true,
    };
    match &choice {
        Choice::SelectAutoAbility { options, .. } => {
            if options.is_empty() {
                return TurnEngine::resume_with_choice(gs, Some(0), None).is_ok();
            }
            let idx = crate::rng::rand_range(options.len()) as i16;
            TurnEngine::resume_with_choice(gs, Some(idx), None).is_ok()
        }
        Choice::SelectLiveSuccess { options, .. } => {
            if options.is_empty() {
                return TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).is_ok();
            }
            let idx = crate::rng::rand_range(options.len());
            TurnEngine::resume_with_choice(gs, None, Some(vec![idx])).is_ok()
        }
        Choice::SelectHeartColor { options, .. }
        | Choice::SelectHeartType { options, .. } => {
            if options.is_empty() {
                return TurnEngine::resume_with_choice(gs, Some(0), None).is_ok();
            }
            let idx = crate::rng::rand_range(options.len()) as i16;
            TurnEngine::resume_with_choice(gs, Some(idx), None).is_ok()
        }
        Choice::SelectTarget { options, target, .. } => {
            let n = options.as_ref().map(|o| o.len()).unwrap_or(2).max(1);
            let idx = (crate::rng::rand_range(n) as i16).min(n as i16 - 1);
            // Choice::SelectTarget's string variants are dispatched by the
            // target name: "choice"/"choice_string"/conditional_optional use
            // the card-indices channel; everything else uses the i16 channel.
            match target.as_str() {
                "choice" | "choice_string" | "conditional_optional" => {
                    TurnEngine::resume_with_choice(gs, None, Some(vec![idx as usize])).is_ok()
                }
                _ => TurnEngine::resume_with_choice(gs, Some(idx), None).is_ok(),
            }
        }
        Choice::SelectPosition { .. } => {
            let idx = crate::rng::rand_range(3) as i16;
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
            // Reconstruct the same option list that handle_choice would show,
            // then pick random legal entries. Using the alias helpers keeps
            // `target` ("self"/"opponent") resolution honest for p2-abilities.
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
            let card_ids = crate::ability::util::zone_cards(player, zone);
            let n_options = filtered_indices
                .as_ref()
                .map(|fi| fi.len())
                .unwrap_or(card_ids.len());
            if n_options == 0 {
                return TurnEngine::resume_with_choice(gs, None, Some(Vec::new())).is_ok();
            }
            // VsAi: the AI never "skips" a mandatory choice; allow_skip paths
            // are human affordances. Auto-pick instead of skipping.
            let _ = allow_skip;
            if *count <= 1 {
                let idx = crate::rng::rand_range(n_options);
                let actual = filtered_indices.as_ref().map(|fi| fi[idx]).unwrap_or(idx);
                TurnEngine::resume_with_choice(gs, None, Some(vec![actual])).is_ok()
            } else {
                let want = (*count).min(n_options);
                let mut picked = Vec::with_capacity(want);
                let mut seen: crate::HashSet<usize> = crate::HashSet::default();
                while picked.len() < want {
                    let idx = crate::rng::rand_range(n_options);
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

/// AI turn: pick a random action and execute it.
pub fn ai_turn(gs: &mut GameState, acts: &[game_setup::Action]) -> bool {
    use crate::game_setup::ActionType;
    // Mulligan phases MUST be concluded, otherwise the AI can keep toggling
    // card selections forever and the game can never reach the main phase.
    // Prefer a Confirm/Skip over the per-card Select actions.
    for a in acts {
        if matches!(
            a.action_type,
            ActionType::ConfirmMulligan | ActionType::SkipMulligan
        ) {
            let _ = game_setup::execute_action(gs, a);
            return true;
        }
    }
    let _ = game_setup::execute_action(gs, &acts[crate::rng::rand_range(acts.len())]);
    true
}

/// How a match is driven.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MatchMode {
    VsAi,
    TwoPlayer,
    AiVsAi,
}

/// Run a full embedded match, converging on the web server's loop sequence.
/// `deck_names` are shown in the selection menus; `cards_of(i)` returns the
/// encoded card numbers of pick `i`; `load_all(i, j)` returns every `Card`
/// needed to build the database for the two selected decks (the union of the
/// two decks' cards — RAM-memory platforms bake this per-deck).
///
/// Returns once a terminal `GameResult` is reached.
pub fn run_embedded_game<U, C, A>(
    ui: U,
    deck_names: &[&str],
    cards_of: C,
    load_all: A,
) -> GameResult
where
    U: PlatformUi,
    C: Fn(usize) -> &'static [&'static str],
    A: FnOnce(usize, usize) -> Vec<Card>,
{
    let mut ui = ui;

    let modes = ["VS AI", "2 Player", "AI vs AI"];
    let mode_idx = select(&mut ui, &modes, "Mode");
    let mode = match mode_idx {
        1 => MatchMode::TwoPlayer,
        2 => MatchMode::AiVsAi,
        _ => MatchMode::VsAi,
    };

    let d1 = select(&mut ui, deck_names, "Your Deck");
    let d2 = if matches!(mode, MatchMode::TwoPlayer) {
        select(&mut ui, deck_names, "P2 Deck")
    } else {
        rng::rand_range(deck_names.len())
    };

    let p1_cards = cards_of(d1);
    let p2_cards = cards_of(d2);
    let all_cards = load_all(d1, d2);
    run_match(&mut ui, p1_cards, p2_cards, all_cards, mode)
}

/// Assemble a match from two deck card-number lists plus the complete `Card`
/// union, then run the shared game loop to a terminal `GameResult`. Exposed
/// separately so host tests can drive a full match without console menus.
pub fn run_match<U: PlatformUi>(
    ui: &mut U,
    p1_cards: &[&str],
    p2_cards: &[&str],
    all_cards: Vec<Card>,
    mode: MatchMode,
) -> GameResult {
    let mut db = Arc::new(CardDatabase::load_or_create(all_cards));

    let nums1: Vec<String> = p1_cards.iter().map(|c| c.to_string()).collect();
    let nums2: Vec<String> = p2_cards.iter().map(|c| c.to_string()).collect();

    let mut pd1 = DeckBuilder::build_deck_from_database(&mut db, nums1).expect("build P1 deck");
    let mut pd2 = DeckBuilder::build_deck_from_database(&mut db, nums2).expect("build P2 deck");
    DeckBuilder::add_default_energy_cards_from_database(&mut pd1, &mut db).ok();
    DeckBuilder::add_default_energy_cards_from_database(&mut pd2, &mut db).ok();
    // Shuffle after adding default energy so the energy deck is fully randomized,
    // matching the web server and 3DS setup order. Previously shuffle was before
    // the add, leaving the default-filled energy cards unshuffled at the bottom.
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

    // ---- the shared, honest loop (mirrors the web server's per-request walk) ----
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
            // Shared routing for every offline target: the engine's
            // `can_player_act` is the single source of truth for who must
            // answer a pending choice (choice_player_id + Choice-embedded
            // picker/target fallback). Every console (DC wasm, 3DS, GBA,
            // Wii) must consult it — no per-platform re-routing. The ad-hoc
            // fixes that lived in web_server and 3DS input are now here.
            let is_ai_choice = match mode {
                MatchMode::AiVsAi => true,
                MatchMode::VsAi => !gs.can_player_act(0),
                MatchMode::TwoPlayer => false,
            };
            if is_ai_choice {
                log::debug!(
                    "[CHOICE_ROUTE] auto-picking for AI pid={:?} mode={:?} choice={:?}",
                    gs.get_pending_choice_player_id(),
                    mode,
                    gs.get_pending_choice()
                );
                if !ai_handle_choice(&mut gs) {
                    break;
                }
            } else {
                log::debug!(
                    "[CHOICE_ROUTE] human-prompted pid={:?} mode={:?} choice={:?}",
                    gs.get_pending_choice_player_id(),
                    mode,
                    gs.get_pending_choice()
                );
                if !handle_choice(ui, &mut gs) {
                    break;
                }
            }
            gs.reset_loop_detection();
            continue;
        }

        // In VsAi the human only plays P1. During RPS, `active_player()` stays the
        // first attacker (P1) for the whole hand, so the AI never gets a turn and
        // the match soft-locks waiting for a P2 RPS choice. The engine routes RPS
        // picks positionally (1st act -> P1, 2nd act -> P2), so once P1 has chosen,
        // the next RPS pick is P2's and must be taken by the AI.
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
    }

    gs.game_result
}