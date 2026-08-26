//! Rule 8.4.7.1 — on a tie where BOTH players win, a player who still has
//! TWO OR MORE cards in their live zone places NOTHING into their success
//! zone. Only a single winning card (or a 1-card zone) moves (8.4.7).
//!
//! Scenario: mirrored boards — each player performs TWO successful copies of
//! the same live card with identical stage support, so the totals are tied by
//! construction (symmetry), both win (8.4.6.2), and each sits on a 2-card
//! live zone at placement time. 8.4.7.1 therefore blocks BOTH placements:
//! neither success zone may receive a card.
//!
//! If a regression ever lets the engine offer/execute a placement here, the
//! strict drain panics on the unexpected SelectLiveSuccess prompt.

use crate::helpers::*;

const LIVE: &str = "PL!-sd1-019-SD"; // START:DASH!!, score 1, needs h01/h03/h06
const MEMBER: &str = "PL!-sd1-001-SD"; // 穂乃果: h01=1, h03=2, h06=1

#[test]
fn tie_with_two_card_live_zones_blocks_all_placements() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");

    // Mirrored boards: three support members per side (direct placement —
    // their own debut triggers are irrelevant here), 40-card decks.
    let m1 = game.id(MEMBER);
    let m2 = game.new_id(MEMBER);
    let m3 = game.new_id(MEMBER);
    game.state.player1.stage.stage = [m1, m2, m3];
    let m4 = game.new_id(MEMBER);
    let m5 = game.new_id(MEMBER);
    let m6 = game.new_id(MEMBER);
    game.state.player2.stage.stage = [m4, m5, m6];

    for seat in [0usize, 1] {
        let player = if seat == 0 {
            &mut game.state.player1
        } else {
            &mut game.state.player2
        };
        player.main_deck.cards.clear();
        player.hand.cards.clear();
        player.waitroom.cards.clear();
        player.success_live_card_zone.cards.clear();
        for _ in 0..40 {
            player.main_deck.cards.push(filler);
        }
    }

    // Each player sets ONE live card from hand…
    let p1_a = game.id(LIVE);
    let p2_a = game.new_id(LIVE);
    game.state.player1.hand.cards.push(p1_a);
    game.state.player2.hand.cards.push(p2_a);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(p1_a);
    game.pass(); // → second attacker's LiveCardSet
    game.set_live_card(p2_a);

    // …and acquires a SECOND live card mid-phase (as if by an effect; direct
    // placement is the established idiom for effect-provided lives).
    let p1_b = game.new_id(LIVE);
    let p2_b = game.new_id(LIVE);
    game.state.player1.live_card_zone.cards.push(p1_b);
    game.state.player2.live_card_zone.cards.push(p2_b);
    assert_eq!(game.state.player1.live_card_zone.cards.len(), 2);
    assert_eq!(game.state.player2.live_card_zone.cards.len(), 2);

    // Run the live round to completion, draining prompts STRICTLY: a
    // SelectLiveSuccess prompt here would mean 8.4.7.1 broke.
    let mut guard = 0;
    while game.state.current_turn_phase == rabuka_engine::game_state::TurnPhase::Live && guard < 24 {
        guard += 1;
        game.pass();
        while game.has_pending_choice() {
            match game.pending_choice_type().as_deref() {
                Some("SelectLiveSuccess") => panic!(
                    "8.4.7.1: a 2-card live zone must NOT be offered a success placement"
                ),
                Some("SelectAutoAbility") => {
                    let n = game.pending_choice_count();
                    let idxs: Vec<usize> = (0..n.max(1) as usize).collect();
                    game.select_indices(&idxs);
                }
                _ => game.select_indices(&[]),
            }
        }
    }

    assert_ne!(
        game.state.current_turn_phase,
        rabuka_engine::game_state::TurnPhase::Live,
        "flow must cross victory determination"
    );

    // THE pin: mirrored successes tied the totals, but 2-card zones place
    // nothing on a both-win tie.
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        0,
        "8.4.7.1: P1's 2-card zone places nothing despite winning the tie"
    );
    assert_eq!(
        game.state.player2.success_live_card_zone.cards.len(),
        0,
        "8.4.7.1: P2's 2-card zone places nothing despite winning the tie"
    );
}
