/// Live Success with sequential draw+discard on both sides.
/// Tests that the game advances past LiveVictoryDetermination even when
/// both players' LiveSuccess abilities create discard choices.
/// Regression test for live_success_p2_fired infinite loop.
use crate::helpers::*;

fn drain_choices(game: &mut TestGame) {
    while game.has_pending_choice() {
        let json = game.state.get_pending_choice_json();
        let zone = json
            .as_ref()
            .and_then(|v| v.get("zone"))
            .and_then(|v| v.as_str());
        if zone == Some("hand") {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }
}

#[test]
fn live_success_both_sides_draw_discard_advances_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!S-bp2-024-L"); // ab#1: LiveSuccess → draw 2, discard 1
    let member = game.id("PL!N-sd1-016-PRproteinbar"); // heart05: 2, abilityless
    let filler = game.id("PL!-sd1-010-SD");

    // Both players have member on stage (heart05 for live success)
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player2.stage.stage = [member, -1, -1];

    // Live card in hand for set_live_card, plus extra for discard cost
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.hand.cards.push(filler);
    game.state.player2.hand.cards.push(filler);

    // Fill decks for draw abilities
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Advance to P1's LiveCardSet phase
    for _ in 0..5 {
        game.pass();
    }
    drain_choices(&mut game);

    game.set_live_card(live);
    drain_choices(&mut game);

    // Set P2's live card directly
    game.state.player2.live_card_zone.cards.push(live);

    // Advance through phases to LiveVictoryDetermination,
    // draining any choices that appear (LiveSuccess abilities).
    for _ in 0..8 {
        if !game.has_pending_choice() {
            game.pass();
        }
        drain_choices(&mut game);
    }

    // After LiveVictoryDetermination resolves, game should advance.
    // If there's an infinite loop, the loop above never exits
    // because P2's LiveSuccess gets re-queued each time.
    assert!(
        !game.state.current_phase.to_string().contains("LiveVictory"),
        "Should have left LiveVictoryDetermination"
    );
}
