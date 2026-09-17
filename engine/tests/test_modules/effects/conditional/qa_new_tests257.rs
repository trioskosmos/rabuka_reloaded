use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Q257: Live phase success zone replacement with 錯覚CROSSROADS (PL!-bp6-024-L).
/// When the live phase places Crossroads in the success zone, its constant
/// replacement ability should fire, letting the player substitute a μ's live
/// card from discard instead.
///
/// This tests the LIVE PHASE path (via move_live_to_success_and_handle_wins)
/// which creates a dummy queue entry — unlike the ability/resolver path used
/// by Q256's Maki-debut scenario.
#[test]
fn live_phase_crossroads_replacement_p1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let crossroads = game.id("PL!-bp6-024-L");
    let muse_live = game.id("PL!-bp3-019-L");

    // Put Crossroads in live_card_zone as if it was set during live card set
    game.state.player1.live_card_zone.cards.push(crossroads);
    // Put a μ's live card in discard
    game.add_to_discard(muse_live);

    // Simulate a won performance with 1 card: triggers replacement logic
    TurnEngine::move_live_to_success_and_handle_wins(
        &mut game.state,
        true,  // player1 won
        false, // player2 lost
    );

    // Should now have a pending SelectCard choice for the replacement
    assert!(
        game.has_pending_choice(),
        "Should have pending choice for replacement"
    );

    // Verify the choice identity metadata is correctly set (the bug fix)
    game.assert_choice_identity(crossroads, "錯覚CROSSROADS", "p1");

    // Verify selection_cards include the μ's live card
    game.assert_selection_contains("PL!-bp3-019-L", "僕らのLIVE 君とのLIFE");

    // Accept the replacement: select the first (and only) μ's live card
    game.select_indices(&[0]);

    // Verify: Crossroads moved to waitroom, μ's live in success zone
    assert!(
        game.state.player1.waitroom.cards.contains(&crossroads),
        "Crossroads should be in waitroom (replacement triggered)"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&muse_live),
        "μ's live card should be in success zone"
    );
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "live_card_zone should be empty after processing"
    );
}

/// Same scenario but the player DECLINES the replacement (skip).
#[test]
fn live_phase_crossroads_replacement_skip_p1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let crossroads = game.id("PL!-bp6-024-L");
    let muse_live = game.id("PL!-bp3-019-L");

    game.state.player1.live_card_zone.cards.push(crossroads);
    game.add_to_discard(muse_live);

    TurnEngine::move_live_to_success_and_handle_wins(&mut game.state, true, false);

    assert!(game.has_pending_choice(), "Should have pending choice");

    // Skip (empty indices = skip for SelectCard with allow_skip=true)
    game.select_indices(&[]);

    // Verify: Crossroads placed in success zone directly, μ's live stays in discard
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&crossroads),
        "Crossroads should be in success zone (no replacement)"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&muse_live),
        "μ's live card should remain in waitroom"
    );
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "live_card_zone should be empty"
    );
}

/// Live phase replacement — Player 2 scenario.
/// Verifies the choice identity and player_id resolution work for P2.
#[test]
fn live_phase_crossroads_replacement_p2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let crossroads = game.id("PL!-bp6-024-L");
    let muse_live = game.id("PL!-bp3-019-L");

    // Put Crossroads in P2's live_card_zone and a μ's live in P2's waitroom
    game.state.player2.live_card_zone.cards.push(crossroads);
    game.state.player2.waitroom.cards.push(muse_live);

    TurnEngine::move_live_to_success_and_handle_wins(
        &mut game.state,
        false, // player1 lost
        true,  // player2 won
    );

    assert!(
        game.has_pending_choice(),
        "Should have pending choice for P2 replacement"
    );

    // Verify identity: card_id, card_name, and choice_player_id = "p2"
    game.assert_choice_identity(crossroads, "錯覚CROSSROADS", "p2");

    // Verify selection_cards include the μ's live in P2's discard
    game.assert_selection_contains("PL!-bp3-019-L", "僕らのLIVE 君とのLIFE");

    // Accept the replacement
    game.select_indices(&[0]);

    assert!(
        game.state.player2.waitroom.cards.contains(&crossroads),
        "P2 Crossroads should be in waitroom"
    );
    assert!(
        game.state
            .player2
            .success_live_card_zone
            .cards
            .contains(&muse_live),
        "P2 μ's live should be in success zone"
    );
}

/// Live phase replacement with Dreamin' Go! Go!! (PL!-bp6-022-L) as the μ's
/// live card in discard.  This was the specific card that triggered the
/// "不明なカード" display bug — ensure it resolves correctly.
#[test]
fn live_phase_crossroads_replacement_with_dreamin() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let crossroads = game.id("PL!-bp6-024-L");
    let dreamin = game.id("PL!-bp6-022-L"); // Dreamin' Go! Go!!

    game.state.player1.live_card_zone.cards.push(crossroads);
    game.add_to_discard(dreamin);

    TurnEngine::move_live_to_success_and_handle_wins(&mut game.state, true, false);

    assert!(game.has_pending_choice(), "Should have pending choice");

    // Verify the choice identity metadata is correct
    game.assert_choice_identity(crossroads, "錯覚CROSSROADS", "p1");

    // Dreamin' Go! Go!! should be in selection_cards with correct name
    game.assert_selection_contains("PL!-bp6-022-L", "Dreamin' Go! Go!!");

    // Accept it
    game.select_indices(&[0]);

    assert!(
        game.state.player1.waitroom.cards.contains(&crossroads),
        "Crossroads should be in waitroom"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&dreamin),
        "Dreamin' Go! Go!! should be in success zone"
    );
}

/// Live phase replacement with MULTIPLE μ's live cards in discard.
/// The player should still be able to pick one.
#[test]
fn live_phase_crossroads_replacement_multiple_targets() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let crossroads = game.id("PL!-bp6-024-L");
    let dreamin = game.id("PL!-bp6-022-L"); // Dreamin' Go! Go!!
    let bokura = game.id("PL!-bp3-019-L"); // 僕らのLIVE 君とのLIFE
    let filler = game.new_id("PL!SP-sd1-023-SD"); // WE WILL!! (live, but NOT μ's)

    game.state.player1.live_card_zone.cards.push(crossroads);
    game.add_to_discard(dreamin);
    game.add_to_discard(bokura);
    game.add_to_discard(filler); // not μ's, should not appear in selection

    TurnEngine::move_live_to_success_and_handle_wins(&mut game.state, true, false);

    assert!(game.has_pending_choice(), "Should have pending choice");

    // Verify choice identity
    game.assert_choice_identity(crossroads, "錯覚CROSSROADS", "p1");

    // Both μ's cards should be selectable
    game.assert_selection_contains("PL!-bp6-022-L", "Dreamin' Go! Go!!");
    game.assert_selection_contains("PL!-bp3-019-L", "僕らのLIVE 君とのLIFE");
    // The non-μ's live card should NOT appear
    game.assert_selection_not_contains("PL!SP-sd1-023-SD");

    // Select the first one (Dreamin' Go! Go!!)
    game.select_indices(&[0]);

    assert!(
        game.state.player1.waitroom.cards.contains(&crossroads),
        "Crossroads should be in waitroom"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&dreamin),
        "Dreamin' Go! Go!! should be in success zone"
    );
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "live_card_zone should be empty"
    );
}

/// Same as Q256 (Maki-debut path / move_cards path) but using Dreamin' Go! Go!!
/// as the μ's live card to catch any issues in the resolver path too.
#[test]
fn maki_debut_crossroads_replacement_with_dreamin() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let maki = game.id("PL!-sd1-006-SD");
    let crossroads = game.id("PL!-bp6-024-L");
    let dreamin = game.id("PL!-bp6-022-L");
    let filler_live = game.new_id("PL!SP-sd1-023-SD");

    game.add_to_hand(maki);
    game.add_to_hand(crossroads);
    game.add_to_discard(dreamin);

    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(filler_live);
    game.add_to_hand(filler_live);

    game.give_energy(9);

    // Play Maki to stage → debut triggers
    game.play_to_stage(maki, MemberArea::Center);

    // The debut ability generates choices in sequence:
    //   1. Optional reveal cost — pick a live card from hand to reveal
    //   2. (Step a: return 1 from success zone to hand — auto, no choice with 1 card)
    //   3. Step b: revealed card placed in success zone → replacement choice
    //
    // Drain all pending choices:
    // Cost choice first, then the replacement choice.
    let mut cost_handled = false;
    let mut replacement_handled = false;
    while game.has_pending_choice() {
        let _before = game.state.ability_queue.len();
        if !cost_handled {
            // First choice: reveal cost
            game.select_indices(&[0]);
            cost_handled = true;
        } else if !replacement_handled {
            // Second choice: replacement — verify Dreamin' Go! Go!! is in selection
            game.assert_selection_contains("PL!-bp6-022-L", "Dreamin' Go! Go!!");
            game.select_indices(&[0]);
            replacement_handled = true;
        } else {
            panic!("Unexpected extra choice");
        }
    }

    assert!(cost_handled, "Reveal cost choice should have appeared");
    assert!(
        replacement_handled,
        "Replacement choice should have appeared"
    );

    // Crossroads should be in waitroom (replaced)
    assert!(
        game.state.player1.waitroom.cards.contains(&crossroads),
        "Crossroads should be in waitroom"
    );
    // Dreamin' Go! Go!! should be in success zone
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&dreamin),
        "Dreamin' should be in success zone"
    );
    // Original filler should be returned to hand
    assert!(
        game.state.player1.hand.cards.contains(&filler_live),
        "Original success zone card should be in hand"
    );
}

/// No valid μ's live targets in discard — replacement should NOT create a choice.
/// The card should be placed in the success zone directly.
#[test]
fn live_phase_crossroads_replacement_no_targets() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let crossroads = game.id("PL!-bp6-024-L");
    let filler = game.new_id("PL!SP-sd1-023-SD"); // live card but NOT μ's

    game.state.player1.live_card_zone.cards.push(crossroads);
    game.add_to_discard(filler); // not μ's — not a valid target

    TurnEngine::move_live_to_success_and_handle_wins(&mut game.state, true, false);

    // No pending choice — replacement can't fire, card goes directly to success zone
    assert!(
        !game.has_pending_choice(),
        "No pending choice expected (no valid μ's targets)"
    );
    assert!(
        game.state
            .player1
            .success_live_card_zone
            .cards
            .contains(&crossroads),
        "Crossroads should be in success zone directly"
    );
}
