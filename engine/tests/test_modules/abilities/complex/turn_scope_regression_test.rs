//! Round-scope regression for the 「このターン」 tracking family.
//!
//! History: `reset_keyword_tracking()` ran on every Active-phase entry —
//! twice per round — truncating first-attacker-window facts before any
//! ライブ開始時/ライブ成功時 condition could read them. Seventeen abilities
//! with the shape 「ライブ開始時：このターン中に…した場合」 depend on those
//! facts surviving across BOTH players' normal phases. These tests pin the
//! corrected lifetime: round-scoped trackers clear only at the turn
//! rollover (victory determination).
//!
//! Behavioral coverage of the same mechanism through real cards lives in
//! remaining_quick_test.rs (cara_q203_*) — those span the windows via real
//! Emma activations. This file pins the tracker lifetimes directly so a
//! future clear-site regression fails HERE, with a message pointing at the
//! cause, instead of as 17 silent card misbehaviors.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn fill(game: &mut TestGame) {
    let filler = game.id_ref("PL!-sd1-010-SD");
    fill_decks(game, filler);
}

/// A member debuted during the FIRST attacker's window must still be
/// visible in round-scoped tracking after the SECOND attacker's Active
/// phase (the historical truncation point) and at live start.
#[test]
fn first_attacker_window_facts_survive_second_active_phase_and_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!S-bp2-001-R"); // Aqours Chika
    let _filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    fill(&mut game);

    // FIRST ATTACKER'S WINDOW: debut a member from hand.
    game.add_to_hand(member);
    game.give_energy(30);
    game.play_to_stage(member, MemberArea::Center);
    assert!(
        game.state
            .cards_appeared_this_turn
            .contains(&member),
        "sanity: debut records the appearance"
    );
    assert_eq!(
        game.state.player1.debut_count_this_turn, 1,
        "sanity: debut counter incremented"
    );

    // Cross into the SECOND ATTACKER'S normal phase — advance_phase enters
    // Active and calls reset_keyword_tracking(), which historically wiped
    // these facts mid-round.
    advance_to_live_card_set_p1(&mut game);
    assert!(
        game.state
            .cards_appeared_this_turn
            .contains(&member),
        "appearance must survive the second attacker's Active phase \
         (「このターン」 spans the whole round)"
    );
    assert_eq!(
        game.state.player1.debut_count_this_turn, 1,
        "debut counter must survive the second attacker's Active phase"
    );

    // And still be readable when ライブ開始時 conditions evaluate.
    game.add_to_hand(live);
    game.set_live_card(live);
    game.pass();
    game.pass();
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            break;
        }
    }
    assert!(
        game.state
            .cards_appeared_this_turn
            .contains(&member),
        "appearance must be visible at live start"
    );
}

/// The OTHER side of the contract: round-scoped facts DO clear at the true
/// turn boundary. Without this, "never cleared" would also pass the test
/// above and leak state across rounds.
#[test]
fn round_scoped_trackers_clear_at_turn_rollover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!S-bp2-001-R");
    let _filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    fill(&mut game);

    game.add_to_hand(member);
    game.add_to_hand(live);
    game.give_energy(30);
    game.play_to_stage(member, MemberArea::Center);
    assert!(
        !game.state.cards_appeared_this_turn.is_empty(),
        "sanity: something was recorded"
    );

    // Drive through the live into the NEXT round's first normal phase.
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    // LiveCardSetFAN → SAN → performances → victory determination → next FAN.
    for _ in 0..8 {
        game.pass();
        let mut guard = 0;
        while game.has_pending_choice() && guard < 20 {
            guard += 1;
            if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
                game.select_indices(&[]);
            } else {
                break;
            }
        }
    }

    assert!(
        game.state.turn_number >= 2,
        "sanity: the turn rolled over"
    );
    assert!(
        game.state.cards_appeared_this_turn.is_empty(),
        "appearances clear at the turn rollover"
    );
    assert_eq!(
        game.state.player1.debut_count_this_turn, 0,
        "debut counters clear at the turn rollover"
    );
    assert!(
        game.state.turn_state_changes.is_empty(),
        "activation history clears at the turn rollover"
    );
}
