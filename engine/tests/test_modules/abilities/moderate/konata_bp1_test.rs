/// Tests for PL!N-bp1-006-R+ (近江彼方 / Kanata Konoe) — Q77: Nijigasaki appearance → activate 2 energy
///
/// The card prints TWO 起動 abilities; runtime order (see [ACTIVATE_CHECK] traces):
///   idx 0: {{E}}{{E}}：カードを1枚引く。           (pay-2-energy draw)
///   idx 1: 手札を1枚控え室に置く：このターン、自分のステージに『虹ヶ咲』の
///          メンバーが登場している場合、エネルギーを2枚アクティブにする。
///
/// Q77: Members that appeared this turn satisfy the condition (even if it's the
///      ability's own card — 近江彼方 is a 虹ヶ咲 member herself).
use crate::helpers::*;

/// Runtime index of the discard→activate-2-energy 起動 ability.
const ENERGY_ABILITY_IDX: usize = 1;

fn setup_konata(game: &mut TestGame, hand_fillers: usize, energy: usize) -> i16 {
    let db_handle = game.state.card_database.clone();
    let _ = db_handle;
    let konata = game.id("PL!N-bp1-006-R\u{ff0b}"); // fullwidth ＋
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    for _ in 0..hand_fillers {
        game.state.player1.hand.cards.push(filler);
    }
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(energy);
    konata
}

/// Q77: The ability's own card appeared this turn → condition passes.
#[test]
fn konata_q77_self_appearance_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = setup_konata(&mut game, 1, 4);

    // Mark konata as "appeared this turn" — she's a 虹ヶ咲 member herself
    game.state.cards_moved_this_turn.push(konata);

    let active_before = game.state.player1.energy_zone.active_count();

    game.activate_ability_index(konata, ENERGY_ABILITY_IDX);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // ab should fire: discard 1 from hand → activate 2 energy
    // Net active change: +2
    let active_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        active_after,
        active_before + 2,
        "ab#1 should activate 2 energy when a 虹ヶ咲 member appeared this turn"
    );
    // One filler discarded from hand to waitroom
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "Discard cost should put 1 card in waitroom"
    );
}

/// Negative: no member appeared this turn → effect condition fails.
#[test]
fn konata_no_appearance_condition_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = setup_konata(&mut game, 1, 4);

    let active_before = game.state.player1.energy_zone.active_count();

    game.activate_ability_index(konata, ENERGY_ABILITY_IDX);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // No member appeared this turn → condition fails, no energy activated
    // (the discard cost may still be charged — printed text pays it first).
    let active_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        active_after, active_before,
        "No energy change when no 虹ヶ咲 member appeared this turn"
    );
}

/// Use limit (ターン1回) blocks second activation of the same ability.
#[test]
fn konata_use_limit_blocks_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = setup_konata(&mut game, 3, 6);
    game.state.cards_moved_this_turn.push(konata);

    // First activation
    let active_before = game.state.player1.energy_zone.active_count();
    game.activate_ability_index(konata, ENERGY_ABILITY_IDX);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let active_after_first = game.state.player1.energy_zone.active_count();
    assert_eq!(
        active_after_first,
        active_before + 2,
        "First activation should activate 2 energy"
    );

    // Second activation of the SAME ability should be blocked by use_limit
    let result = game.try_activate_ability_index(konata, ENERGY_ABILITY_IDX);
    assert!(
        result.is_err(),
        "Second activation should be blocked by use_limit"
    );
}
