use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member

#[test]
fn sp_bp1_009_activation_draws_one_discards_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp1-009-R");
    game.state.player1.stage.stage[0] = me;
    game.give_energy(1);

    // Pre-existing hand card, plus a known deck order.
    let held = game.new_id(FILLER);
    game.state.player1.hand.cards.push(held);
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);

    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.activate_ability(me);
    assert!(
        game.has_pending_choice(),
        "hand discard prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard for the discard"
    );
    // Discard 1 from hand — pick index 0 (whichever card that is).
    game.select_indices(&[0]);

    assert_eq!(game.state.player1.hand.cards.len(), 1, "net hand size stays 1");
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 1,
        "exactly one card was discarded to the waitroom"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&drawn),
        "the stocked deck card was drawn"
    );
}
