/// Shared helpers for BP07 CLEAN-G4 wait-immunity tests.
///
/// The immunity (松浦果南 PL!S-bp7-003-R＋ ab#1 option 1) protects the owner's
/// Aqours members (blade ≤ 3) from being put to WAIT by the OPPONENT's effects.
///
/// The pattern: PLAYER2 establishes immunity on their own 果南, then PLAYER1's
/// wait ability (the one under test) targets player2's protected member → the
/// wait must be blocked. Player2 must play the immunity card, so the helper
/// flips the active player.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

pub const KANAN: &str = "PL!S-bp7-003-R\u{ff0b}"; // 松浦果南 — Aqours, cost 4, blade 2

pub fn set_active(game: &mut TestGame, p1_active: bool) {
    game.state.player1.is_first_attacker = p1_active;
    game.state.player2.is_first_attacker = !p1_active;
}

/// Player2 plays 松浦果南 (debut) and picks option 1 (wait-immunity), then the
/// active player is restored to player1. Returns player2's protected 果南 id.
pub fn p2_establish_wait_immunity(game: &mut TestGame) -> i16 {
    set_active(game, false);
    let kanan = game.id(KANAN);
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player2.main_deck.cards.clear();
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(kanan);
    for _ in 0..10 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.add_active(10);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(kanan),
        None,
        Some(MemberArea::Center),
        None,
    )
    .expect("p2 plays 松浦果南");

    // Answer the debut choice: option 1 = wait-immunity restriction.
    let mut guard = 0;
    while game.state.has_pending_choice() && guard < 20 {
        guard += 1;
        let is_card = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard { .. }
            )
        });
        if is_card {
            game.select_indices(&[0]);
        } else {
            game.select_choice_option(0);
        }
    }
    set_active(game, true);
    kanan
}

/// Player1 plays 松浦果南 (debut) and picks option 1 on their OWN stage; returns
/// player1's protected 果南 id.
pub fn p1_establish_wait_immunity(game: &mut TestGame) -> i16 {
    let kanan = game.id(KANAN);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(kanan);
    game.give_energy(30);
    game.play_to_stage(kanan, MemberArea::Center);
    let mut guard = 0;
    while game.state.has_pending_choice() && guard < 20 {
        guard += 1;
        let is_card = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard { .. }
            )
        });
        if is_card {
            game.select_indices(&[0]);
        } else {
            game.select_choice_option(0);
        }
    }
    kanan
}

pub fn is_waited(game: &TestGame, id: i16) -> bool {
    game.state.mods.get_orientation_modifier(id).as_deref() == Some("wait")
}
