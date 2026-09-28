use crate::helpers::*;

#[test]
fn wait_other_group_draw_self_excluded_no_other_group_cost_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[2] = filler;

    game.state.player1.hand.cards.push(filler);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Activate ability
    let result = game.try_activate_ability(emma);

    // Cost: wait a ????member other than self.
    // With only ????(a ????member) on stage and exclude_self=true,
    // no valid candidates — the mandatory cost is unpayable, so activation
    // is refused outright (Rule 9.4.2.3/Q56) and the ability never draws.
    assert!(
        result.is_err(),
        "unpayable wait cost must refuse activation, got {:?}",
        result
    );

    // Drain any pending choices
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    // Cost should fail since exclude_self leaves no candidates.
    // The failed cost should not proceed to draw.
    let hand_count = game.state.player1.hand.cards.len();
    // hand started with 1 filler, never drew because ability cost failed
    assert_eq!(
        hand_count, 1,
        "No draw happened because cost couldn't be paid (got {})",
        hand_count
    );
}

#[test]
fn wait_other_group_draw_opponent_nijigasaki_is_not_a_cost_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    let opponent_niji = game.id("PL!N-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [emma, filler, -1];
    game.state.player2.stage.stage = [opponent_niji, -1, -1];
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let result = game.try_activate_ability(emma);
    assert!(result.is_err(), "Q163: opponent member cannot pay own-stage cost");
    assert!(
        !game.has_pending_choice(),
        "Q163: no target prompt when only an opponent member qualifies"
    );
    assert_ne!(
        game.state.mods.get_orientation_modifier(opponent_niji),
        Some("wait"),
        "Q163: opponent member must remain unchanged"
    );
}

#[test]
fn wait_other_group_draw_other_group_member_pays_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let emma = game.id("PL!N-bp3-008-R\u{ff0b}");
    // 虹ヶ咲 member: any 虹ヶ咲 series member card
    let niji = game.id("PL!N-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[1] = emma;
    game.state.player1.stage.stage[2] = niji;

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    // Activate Emma's ability
    game.activate_ability(emma);

    // Cost: wait a 虹ヶ咲 member other than self — niji is the ONLY candidate,
    // so the engine auto-applies the wait (single legal target, no prompt needed).
    // Strict: verify the wait was actually applied to niji (not emma, not filler).
    let niji_waited = game
        .state
        .mods
        .get_orientation_modifier(niji) == Some("wait");
    assert!(niji_waited, "niji (only 虹ヶ咲 candidate) must be auto-waited as cost");
    let emma_waited = game
        .state
        .mods
        .get_orientation_modifier(emma) == Some("wait");
    assert!(!emma_waited, "exclude_self: Emma herself must NOT be waited");
    let filler_waited = game
        .state
        .mods
        .get_orientation_modifier(filler) == Some("wait");
    assert!(!filler_waited, "non-虹ヶ咲 filler must NOT be waited");

    let hand_count = game.state.player1.hand.cards.len();
    assert!(hand_count > 0, "Should have drawn 1 card after cost");
}
