/// Live Success with sequential draw+discard on both sides.
/// Tests that the game advances past LiveVictoryDetermination even when
/// both players' LiveSuccess abilities create discard choices.
/// Regression test for live_success_p2_fired infinite loop.
///
/// Beyond "doesn't hang", this verifies the abilities' actual effect:
/// 「カードを2枚引き、手札を1枚控え室に置く」→ net +1 hand and +1 waitroom
/// card per side.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::Phase;

fn drain_choices(game: &mut TestGame) {
    // Bounded: an unbounded drain cannot prove termination.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 40 {
        guard += 1;
        match game.get_pending_choice() {
            // The ability's mandatory「手札を1枚控え室に置く」step.
            Choice::SelectCard { zone, count, .. } if zone == "hand" && *count >= 1 => {
                game.select_indices(&[0]);
            }
            _ => game.select_indices(&[]),
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
    // Baselines are captured the moment determination begins — after ALL
    // phase-advance draws (live-zone refill etc.), before any LiveSuccess
    // effect fires.
    let mut p1_hand_before = None;
    let mut p2_hand_before = None;
    let mut p1_wait_before = None;
    let mut p2_wait_before = None;
    for _ in 0..8 {
        if !game.has_pending_choice() {
            game.pass();
        }
        eprintln!(
            "[loop] phase={:?} tphase={:?} pending={}",
            game.state.current_phase,
            game.state.current_turn_phase,
            game.pending_choice_summary()
        );
        if game
            .state
            .current_phase
                == Phase::LiveVictoryDetermination
            && p1_hand_before.is_none()
        {
            p1_hand_before = Some(game.state.player1.hand.cards.len());
            p2_hand_before = Some(game.state.player2.hand.cards.len());
            p1_wait_before = Some(game.state.player1.waitroom.cards.len());
            p2_wait_before = Some(game.state.player2.waitroom.cards.len());
        }
        drain_choices(&mut game);
    }
    // Finalize placement, but STOP once the live phase is over — further
    // passes roll into the next round's draw phase and pollute the deltas.
    for _ in 0..3 {
        if game.state.current_phase != Phase::LiveVictoryDetermination
            || game.has_pending_choice()
        {
            break;
        }
        game.pass();
        drain_choices(&mut game);
    }
    // One more pass finalizes placement/turn rollover.
    if !game.has_pending_choice() {
        game.pass();
    }
    // Resolve any trailing ability step (e.g. P1's post-draw discard)
    // triggered by the finalizing pass, so the deltas below measure
    // completed effects only.
    drain_choices(&mut game);

    // Regression core: no infinite re-queue of P2's LiveSuccess.
    assert!(
        !matches!(game.state.current_phase, Phase::LiveVictoryDetermination),
        "Should have left LiveVictoryDetermination"
    );

    // The abilities' actual effects, per side: drew 2, discarded 1.
    // (Baselines were snapshotted the instant determination began — after
    // every phase-advance draw: main-deck draw, live-zone refills for BOTH
    // players — so +1 net is purely the ability's 2-draw/1-discard.)
    let p1_hand_before = p1_hand_before.expect("victory determination reached");
    let p2_hand_before = p2_hand_before.expect("victory determination reached");
    let p1_wait_before = p1_wait_before.expect("victory determination reached");
    let p2_wait_before = p2_wait_before.expect("victory determination reached");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        p1_hand_before + 1,
        "P1: draw 2 - discard 1 = net +1"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before + 1,
        "P2: draw 2 - discard 1 = net +1"
    );
    // Waitroom gains per side: the ability's discarded hand card PLUS the
    // live card itself — its 常時 ab#0 forbids it from the success zone,
    // so victory cleanup routes it to the waitroom.
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        p1_wait_before + 2,
        "P1: ability discard + restricted live card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live)
            && !game.state
                .player1
                .success_live_card_zone
                .cards
                .contains(&live),
        "restricted live card must not enter the success zone"
    );
    assert_eq!(
        game.state.player2.waitroom.cards.len(),
        p2_wait_before + 2,
        "P2: ability discard + restricted live card"
    );
}
