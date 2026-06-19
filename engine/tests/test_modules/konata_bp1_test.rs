/// Tests for PL!N-bp1-006-R+ (近江彼方 / Kanata Konoe) — Q77: Nijigasaki appearance → activate 2 energy
///
/// ab#0:
///   {{kidou.png|起動}}{{turn1.png|ターン1回}}手札を1枚控え室に置く：
///   このターン、自分のステージに『虹ヶ咲』のメンバーが登場している場合、エネルギーを2枚アクティブにする。
///
/// Q77: Members that appeared this turn satisfy the condition (even if it's the
///      ability's own card — 近江彼方 is a 虹ヶ咲 member herself).
use crate::helpers::*;

/// Q77: The ability's own card appeared this turn → condition passes.
#[test]
fn konata_q77_self_appearance_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}"); // fullwidth ＋
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    // Hand: a card to discard for cost
    game.state.player1.hand.cards.push(filler);

    // Energy: give energy for activating
    game.give_energy(4);

    // Mark konata as "appeared this turn" — she's a 虹ヶ咲 member herself
    game.state.cards_moved_this_turn.insert(konata);

    let active_before = game.state.player1.energy_zone.active_energy_count;

    game.activate_ability(konata);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // ab#0 should fire: discard 1 from hand → activate 2 energy
    // Net active change: +2
    let active_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        active_after,
        active_before + 2,
        "ab#0 should activate 2 energy when a 虹ヶ咲 member appeared this turn"
    );
    // One filler discarded from hand to waitroom
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "Discard cost should put 1 card in waitroom"
    );
}

/// Negative: no member appeared this turn → condition fails.
#[test]
fn konata_no_appearance_condition_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    let active_before = game.state.player1.energy_zone.active_energy_count;

    game.activate_ability(konata);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // No member appeared this turn → condition fails, nothing happens
    let active_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        active_after, active_before,
        "No energy change when no 虹ヶ咲 member appeared this turn"
    );
}

/// Use limit (ターン1回) blocks second activation.
#[test]
fn konata_use_limit_blocks_second_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(6);

    game.state.cards_moved_this_turn.insert(konata);

    // First activation
    game.activate_ability(konata);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let active_after_first = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        active_after_first, 2,
        "First activation should activate 2 energy"
    );

    // Second activation should fail
    let result = game.try_activate_ability(konata);
    assert!(
        result.is_err(),
        "Second activation should be blocked by use_limit"
    );
}
