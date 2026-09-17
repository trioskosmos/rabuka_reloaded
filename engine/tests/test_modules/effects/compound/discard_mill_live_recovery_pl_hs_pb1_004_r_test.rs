use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_hs_pb1_004_r_paid_discard_mills_three_recovers_cerise_bouquet_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-pb1-004-R");
    game.add_to_hand(me);
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let sblive = game.new_id("PL!HS-PR-010-PR");
    game.state.player1.waitroom.cards.push(sblive);
    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "top 3 cards milled to the waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&sblive),
        "『スリーズブーケ』 live recovered to hand"
    );
}

#[test]
fn pl_hs_pb1_004_r_declined_discard_skips_mill_and_live_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-pb1-004-R");
    game.add_to_hand(me);
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let sblive = game.new_id("PL!HS-PR-010-PR");
    game.state.player1.waitroom.cards.push(sblive);
    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::Center);
    assert!(
        game.has_pending_choice(),
        "optional hand-discard cost must be offered"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected skippable SelectCard cost prompt"
    );
    game.select_option(1);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined -> no mill"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&sblive),
        "declined -> live stays in the waitroom"
    );
}
