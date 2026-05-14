/// Tests for 近江彼方 (PL!N-bp1-006-R+) — Activation ability (ab#0):
///
/// {{kidou.png|起動}}{{turn1.png|ターン1回}}手札を1枚控え室に置く：
/// このターン、自分のステージに「虹ヶ咲」のメンバーが登場している場合、
/// エネルギーを2つアクティブにする。
///
/// Q77: Members that debuted this turn but left the stage still satisfy
///      the condition (debut count, not current presence).
use crate::helpers::*;

/// Q77: If a 虹ヶ咲 member debuted this turn (even if no longer on stage),
/// the condition passes and 2 energy are activated.
#[test]
fn konata_q77_debuted_this_turn_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let niji_member = game.id("PL!N-PR-009-PR"); // 優木せつ菜, cost=2, group=虹ヶ咲

    // Stage: 近江彼方
    game.state.player1.stage.stage[1] = konata;

    // Hand: a card to discard for cost
    game.state.player1.hand.cards.push(niji_member);

    // Energy: some cards for activation test
    game.give_energy(4);

    // Record a debut (simulating a 虹ヶ咲 member played to stage this turn)
    game.state.player1.debut_count_this_turn = 1;

    let active_before = game.state.player1.energy_zone.active_energy_count;

    game.activate_ability(konata);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Ab#0: cost 2E, draw 1. Ab#1: cost discard hand→waitroom, activate 2E (condition PASSED).
    // Net active change: -2 (ab#0 cost) +2 (ab#1 activation) = 0.
    // Hand: originally 1 card. Ab#0 draws +1→2. Ab#1 discards 1→1. Net: 1.
    let active_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        active_after, active_before,
        "Ab#0 cost 2E + Ab#1 activate 2E = net 0 change"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&niji_member),
        "Ab#1 cost (discard niji_member) was paid"
    );
}

/// Negative: activate ability before any member debuted — condition fails.
#[test]
fn konata_no_debut_condition_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let konata = game.id("PL!N-bp1-006-R\u{ff0b}");
    let niji_member = game.id("PL!N-PR-009-PR");

    game.state.player1.stage.stage[1] = konata;
    game.state.player1.hand.cards.push(niji_member);
    game.give_energy(4);

    let active_before = game.state.player1.energy_zone.active_energy_count;

    game.activate_ability(konata);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // No member debuted → debut_count_this_turn = 0, condition fails
    // Cost: 2E paid (active drops by 2). Condition failed → no activation.
    let active_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        active_before - active_after,
        2,
        "Condition fails: 2E cost paid, no activation, net -2"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&niji_member),
        "Cost card should be in waitroom (discard cost was paid)"
    );
}
