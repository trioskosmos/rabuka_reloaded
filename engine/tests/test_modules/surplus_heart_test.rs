/// Tests for 余剰ハート (surplus heart) mechanics.
use crate::helpers::*;

#[test]
fn bella_q173_two_lives_succeed_both_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bella1 = game.new_id("PL!N-bp3-027-L");
    let bella2 = game.new_id("PL!N-bp3-027-L");
    let emma = game.id("PL!N-pb1-008-R");
    let ayumu = game.id("PL!N-PR-003-PR");
    let hasu = game.id("PL!HS-pb1-023-N");
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    // Put 2 energy in energy_deck so Bella can move them
    let energy = game.id("LL-E-001-SD");
    game.state.player1.energy_deck.cards.push(energy);
    game.state.player1.energy_deck.cards.push(energy);

    game.state.player1.stage.stage = [emma, ayumu, hasu];
    game.state.player1.hand.cards.push(bella1);
    game.state.player1.hand.cards.push(bella2);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(bella1);
    game.set_live_card(bella2);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        2,
        "Both Bella lives should trigger energy fix (Q173)"
    );
    assert!(!game.has_pending_choice());
}
