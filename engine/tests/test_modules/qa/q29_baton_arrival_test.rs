//! Rule 9.6.2.1.2.1 / Q29 — a member played THIS turn cannot be sent to the
//! waitroom by a baton touch on that same turn; from the NEXT turn the same
//! baton touch becomes legal.
//!
//! kasumi_test pins the `deployed_this_turn` BOOKKEEPING; here we drive the
//! REAL action pipeline (`try_play_to_stage_for`) so the prohibition itself
//! is what's under test:
//!   1. same turn: second play onto her area must Err, member stays.
//!   2. next turn: the identical play succeeds — baton touch nets costs
//!      (Q24), the departing member lands in the waitroom.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FIRST: &str = "PL!-sd1-010-SD"; // cost 4, no abilities
const SECOND: &str = "PL!N-PR-008-PR"; // cost 9

#[test]
fn q29_baton_touch_blocked_on_arrival_turn_allowed_next_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let first = game.id(FIRST);
    let second = game.id(SECOND);
    let filler = game.id("PL!-sd1-010-SD");

    fill_decks(&mut game, filler);
    game.give_energy(20);
    game.state.player1.hand.cards.push(first);
    game.state.player1.hand.cards.push(second);

    // Turn 1: deploy the first member to LeftSide.
    game.try_play_to_stage_for(Side::P1, first, MemberArea::LeftSide)
        .expect("first deployment is a plain play");
    game.drain_auto_ability_choices();
    assert_eq!(game.state.player1.stage.stage[0], first);

    // SAME turn: baton touch attempt onto her occupied area must fail.
    let err = game
        .try_play_to_stage_for(Side::P1, second, MemberArea::LeftSide)
        .expect_err("Q29: baton touch on an arrival-turn member must be rejected");
    log::debug!("[Q29_TEST] rejection reason: {}", err);

    assert_eq!(
        game.state.player1.stage.stage[0], first,
        "the protected member stays on stage"
    );
    assert!(
        game.state.player1.hand.cards.contains(&second),
        "the incoming member stays in hand"
    );

    // Turn 2: rollover clears deployed_this_turn — the SAME play now works.
    // Drive back into P1's own Main phase.
    let mut guard = 0;
    while !(game.state.current_phase == rabuka_engine::game_state::Phase::Main
        && game.state.current_turn_phase == rabuka_engine::game_state::TurnPhase::FirstAttackerNormal
        && !game.state.player1.deployed_this_turn.contains(&first))
        && guard < 20
    {
        guard += 1;
        game.pass();
        while game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }

    game.try_play_to_stage_for(Side::P1, second, MemberArea::LeftSide)
        .expect("next turn: the identical baton touch is legal");

    assert_eq!(
        game.state.player1.stage.stage[0], second,
        "the incoming member takes the area"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&first),
        "Q24/Q141: the departing member goes to the waitroom"
    );
}
