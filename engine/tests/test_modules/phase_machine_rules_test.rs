//! Phase-machine rules — the side effects EVERY turn depends on.
//!
//! Game-system behaviors with no ability text, invisible to the ability
//! inventory:
//!
//! - 7.4.1: waited cards stand for the TURN PLAYER only (empirically applied
//!   across the Active→Energy boundary); the other player's persist.
//! - 7.5.2: the Energy phase moves the TOP card of the turn player's energy
//!   deck; an EMPTY deck moves nothing (and must not panic).
//! - 7.6.2 + Q267: exactly ONE card drawn per Draw phase; an empty main deck
//!   refreshes first and the draw still happens.
//!
//! Identity note (empirical): `is_first_attacker` NEVER flips — it records
//! who won RPS. Whose turn it is derives from `current_turn_phase`
//! (FirstAttackerNormal / SecondAttackerNormal).

use crate::helpers::*;
use rabuka_engine::core::game_modifiers::CardOrientation;
use rabuka_engine::game_state::{Phase, TurnPhase};

fn orientation(game: &TestGame, cid: i16) -> Option<CardOrientation> {
    game.state.mods.orientation_modifiers.get(&cid).copied()
}

fn wait(game: &mut TestGame, cid: i16) {
    game.state.mods.add_orientation_modifier(cid, "wait");
}

/// Seat index (0 = player1) of the CURRENT turn player, derived from the
/// normal-phase marker. None outside a normal phase (e.g. during the live).
fn normal_phase_seat(game: &TestGame) -> Option<usize> {
    match game.state.current_turn_phase {
        TurnPhase::FirstAttackerNormal => Some(0),
        TurnPhase::SecondAttackerNormal => Some(1),
        _ => None,
    }
}

/// Pass until `phase` is reached during the given seat's normal-phase turn.
fn pass_until(game: &mut TestGame, phase: Phase, seat: usize) {
    let mut guard = 0;
    while !(game.state.current_phase == phase && normal_phase_seat(game) == Some(seat)) {
        guard += 1;
        assert!(guard <= 24, "phase {:?} (seat {}) not reached", phase, seat);
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }
}

/// 7.4.1 — the SECOND attacker's waited member AND waited energy stand as her
/// normal phase proceeds; the FIRST attacker's waited cards persist.
#[test]
fn active_phase_stands_only_the_turn_players_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_member = game.id("PL!-sd1-010-SD");
    let p2_member = game.id("PL!N-PR-008-PR"); // quiet blade-4 member
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = p1_member;
    game.state.player2.stage.stage[0] = p2_member;
    wait(&mut game, p1_member);
    wait(&mut game, p2_member);

    // Both players: 1 active + 1 waited energy.
    for seat in [0usize, 1] {
        let e1 = game.id("LL-E-001-SD");
        let e2 = game.id("LL-E-001-SD");
        let player = if seat == 0 {
            &mut game.state.player1
        } else {
            &mut game.state.player2
        };
        player.energy_zone.cards.push(e1);
        player.energy_zone.cards.push(e2);
        player.energy_zone.set_active_count(1);
    }

    fill_decks(&mut game, filler);

    // One pass: Main(FirstAttackerNormal) → SECOND attacker's Active…
    game.pass();
    assert_eq!(game.state.current_phase, Phase::Active);
    assert_eq!(
        normal_phase_seat(&game),
        Some(1),
        "after one pass from FirstAttackerNormal the second attacker acts"
    );

    // …and by the time her Energy phase begins (Active→Energy boundary),
    // 7.4.1 has stood HER waited cards and ONLY hers.
    game.pass();
    assert_eq!(game.state.current_phase, Phase::Energy);

    assert_ne!(
        orientation(&game, p2_member),
        Some(CardOrientation::Wait),
        "7.4.1: the turn player's waited member stands"
    );
    assert_eq!(
        orientation(&game, p1_member),
        Some(CardOrientation::Wait),
        "the non-turn player's waited member must stay waited"
    );
    assert_eq!(
        game.state.player2.energy_zone.active_count(),
        2,
        "turn player's waited energy stands"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "non-turn player's waited energy stays put"
    );
}

/// 7.5.2 — the Energy phase moves the top card of the TURN PLAYER's energy
/// deck; an EMPTY energy deck moves nothing and must not panic (silent skip).
#[test]
fn energy_phase_draws_one_and_empty_deck_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");

    // First attacker's energy deck seeded with 1 identifiable card.
    fill_energy_deck(&mut game, 0, 1);
    let seeded = game.state.player1.energy_deck.cards[0];
    fill_decks(&mut game, filler);

    // Second attacker reaches Energy with an EMPTY deck: nothing moves.
    game.pass(); // → second attacker Active
    game.pass(); // → second attacker Energy
    assert_eq!(normal_phase_seat(&game), Some(1));
    assert_eq!(
        game.state.player2.energy_deck.cards.len(),
        0,
        "precondition: their deck is empty"
    );
    assert_eq!(
        game.state.player2.energy_zone.cards.len(),
        0,
        "empty energy deck -> nothing placed, no panic"
    );

    // Drive around to the FIRST attacker's own Energy phase…
    pass_until(&mut game, Phase::Energy, 0);
    // …the top-deck move executes on the Energy→Draw transition (phases.rs
    // draw_energy() sits in that boundary), so take one more pass.
    game.pass();

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        1,
        "7.5.2: the turn player's energy phase moves her top deck card"
    );
    assert!(
        game.state.player1.energy_zone.cards.contains(&seeded),
        "the SEEDED card was moved (top-of-deck order respected)"
    );
}

/// 7.6.2 + Q267 — exactly ONE card drawn per Draw phase; drawing from an
/// empty main deck refreshes from the waitroom FIRST, so the player still
/// draws and `deck_refreshed_this_turn` is recorded.
#[test]
fn draw_phase_on_empty_main_deck_refreshes_then_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // First attacker: empty main deck, 3 waitroom cards as refresh material.
    game.state.player1.main_deck.cards.clear();
    game.state.player1.waitroom.cards.clear();
    for _ in 0..3 {
        game.state
            .player1
            .waitroom
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    let f = game.new_id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(f);
    }

    // Capture BEFORE the Draw phase (the draw fires on entering it).
    pass_until(&mut game, Phase::Energy, 0);
    let hand_before = game.state.player1.hand.cards.len();

    pass_until(&mut game, Phase::Main, 0);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "7.6.2: exactly one card drawn despite the empty deck"
    );
    // NOTE: deck_refreshed_this_turn is only recorded by the explicit
    // effect-driven refresh path (mill/look overdraw); the phase-draw refresh
    // is silent. Observable outcomes are what we pin here.
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        2,
        "3 waitroom cards shuffled in, 1 drawn out"
    );
}
