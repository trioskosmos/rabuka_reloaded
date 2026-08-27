/// Q164: 天王寺璃奈 (PL!N-bp3-009-R＋) — LiveStart: put 2 member cards from
/// YOUR discard to the deck bottom. Only your own discard qualifies.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

#[test]
fn rina_bp3_q164_select_from_own_discard_not_opponents() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let rina = game.id("PL!N-bp3-009-R＋");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("LL-bp5-001-L");

    fill_decks(&mut game, filler);

    // Two member cards in OUR discard.
    let member_a = game.id("PL!-sd1-002-SD");
    let member_b = game.id("PL!-sd1-005-SD");
    game.state.player1.waitroom.cards.push(member_a);
    game.state.player1.waitroom.cards.push(member_b);

    // A DISTINCT copy in the opponent discard must stay untouched.
    let opp_copy = game.new_id("PL!-sd1-002-SD");
    game.state.player2.waitroom.cards.push(opp_copy);

    game.state.player1.stage.stage = [rina, filler, filler];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // Bounded drain: pay the optional selection, two members at a time.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { zone, .. }
                if zone == "discard" || zone == "waitroom" =>
            {
                game.select_indices(&[0]);
                game.select_indices(&[0]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    let deck = &game.state.player1.main_deck.cards;
    let n = deck.len();
    assert!(
        n >= 2 && deck[n - 2..].contains(&member_a) && deck[n - 2..].contains(&member_b),
        "Q164: BOTH own-discard members moved to the DECK BOTTOM, tail={:?}",
        &deck[n.saturating_sub(2)..]
    );
    assert!(
        game.state.player2.waitroom.cards.contains(&opp_copy),
        "Q164: the OPPONENT's identical-card copy stays in THEIR discard"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "own discard emptied by the move"
    );
}