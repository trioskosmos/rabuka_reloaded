/// Tests for PL!N-bp1-006-R+ (近江彼方 / Kanata Konoe) — Q77: Nijigasaki appearance → activate 2 energy
///
/// Card has two abilities (unique_abilities index 8 = ab#1, index 20 = ab#0).
/// The engine stores them in unique_abilities order, so ab#1 is index 0 and
/// is selected first. To test ab#0, we use up ab#1's use_limit first, then
/// the engine falls through to ab#0 on the next activation.
///
/// ab#1 (index 0): {{kidou.png|起動}}{{turn1.png|ターン1回}}{{icon_energy.png|E}}{{icon_energy.png|E}}：カードを1枚引く。
///   cost: 2 energy
///   effect: draw 1 card
///
/// ab#0 (index 1): {{kidou.png|起動}}{{turn1.png|ターン1回}}手札を1枚控え室に置く：
///   このターン、自分のステージに『虹ヶ咲』のメンバーが登場している場合、エネルギーを2枚アクティブにする。
///   cost: discard 1 from hand
///   effect: activate 2 energy (conditional on 虹ヶ咲 appearance this turn)
///
/// Q77: Members that appeared this turn satisfy the condition (even if it's the
///      ability's own card — 近江彼方 is a 虹ヶ咲 member herself).
use crate::helpers::*;

/// Helper: activate the ability and drain choice prompts.
fn activate_and_drain(game: &mut TestGame, konata: i16) {
    game.activate_ability(konata);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// Advance to turn 2 (P1 Main phase).
fn advance_to_turn2(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
    }
}

// ─────────────────────────────────────────────────────────────
//  Tests: 虹ヶ咲 member appeared → condition passes
// ─────────────────────────────────────────────────────────────

/// Q77: The ability's own card (近江彼方, a 虹ヶ咲 member) appeared this turn.
/// First activation burns ab#1's use_limit. Second fires ab#0 → +2 energy.
#[test]
fn q77_self_appearance_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    game.state.cards_moved_this_turn.insert(konata);

    // Burn ab#1 then fire ab#0
    activate_and_drain(&mut game, konata);
    activate_and_drain(&mut game, konata);

    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 4,
        "net: 4-2+2=4 (ab#1 cost 2E, ab#0 activates 2E)"
    );
}

/// A different 虹ヶ咲 member appears on turn 2 → ability fires.
/// The card must be on stage to satisfy the "stage" location check.
#[test]
fn different_niji_member_on_turn2_activates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let niji_other = game.id("PL!N-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    advance_to_turn2(&mut game);

    // Arrange for turn 2: put a different 虹ヶ咲 member on stage
    game.state.player1.stage.stage[0] = niji_other;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    // Mark it as appeared this turn
    game.state.cards_moved_this_turn.insert(niji_other);

    activate_and_drain(&mut game, konata);
    activate_and_drain(&mut game, konata);

    // Turn 2 starts with 8 energy (second give_energy(4) after advance adds to the first 4).
    // ab#1 pays 2E (8→6), ab#0 activates 2E (6→8).
    assert_eq!(
        game.state.player1.energy_zone.active_energy_count, 8,
        "net: 8-2+2=8 — different 虹ヶ咲 member on stage this turn satisfies the condition"
    );
}

// ─────────────────────────────────────────────────────────────
//  Tests: no qualifying member appeared → condition fails
// ─────────────────────────────────────────────────────────────

/// Negative: no member appeared this turn → condition fails.
#[test]
fn no_appearance_condition_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    activate_and_drain(&mut game, konata);

    let active_before = game.state.player1.energy_zone.active_energy_count;
    activate_and_drain(&mut game, konata);
    let active_after = game.state.player1.energy_zone.active_energy_count;

    assert_eq!(
        active_after, active_before,
        "No energy gain when no member appeared this turn"
    );
}

/// A non-虹ヶ咲 member appeared → condition fails (group mismatch).
#[test]
fn non_niji_member_does_not_activate() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let honoka = game.id("PL!-sd1-010-SD"); // μ's member, NOT 虹ヶ咲

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);

    // A non-虹ヶ咲 member appeared this turn
    game.state.cards_moved_this_turn.insert(honoka);

    activate_and_drain(&mut game, konata);

    let active_before = game.state.player1.energy_zone.active_energy_count;
    activate_and_drain(&mut game, konata);
    let active_after = game.state.player1.energy_zone.active_energy_count;

    assert_eq!(
        active_after, active_before,
        "No energy gain when only a non-虹ヶ咲 member appeared"
    );
}

// ─────────────────────────────────────────────────────────────
//  Use limit
// ─────────────────────────────────────────────────────────────

/// Use limit blocks third activation (both ab#1 and ab#0 used this turn).
#[test]
fn use_limit_blocks_both_abilities() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(6);
    game.state.cards_moved_this_turn.insert(konata);

    activate_and_drain(&mut game, konata); // ab#1
    activate_and_drain(&mut game, konata); // ab#0

    let result = game.try_activate_ability(konata);
    assert!(
        result.is_err(),
        "Third activation should fail: both abilities at use_limit"
    );
}
