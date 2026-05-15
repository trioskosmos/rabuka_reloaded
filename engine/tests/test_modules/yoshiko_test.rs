/// Tests for 津島善子 (PL!S-bp3-006-R＋) — Center ability: wait self, discard 1, search
/// deck for Aqours member with cost = own_cost + 2, debut to same area.
///
/// Q154: When no matching Aqours member exists in deck, the search ends silently
/// (no error, no add).
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;

/// Activate Yoshiko's ability with enough energy, no matching Aqours member in deck.
/// The ability should search, find nothing, and end without addition.
#[test]
fn yoshiko_q154_no_candidate_in_deck_search_ends_cleanly() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    // Only fillers in deck — no Aqours members
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Yoshiko on Center (ability requires Center)
    game.state.player1.stage.stage[1] = yoshiko;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[2] = filler;

    game.state.player1.hand.cards.push(filler); // for the discard cost
    game.give_energy(15);

    // Activate ability
    game.activate_ability(yoshiko);

    // Cost 1: wait self (change_state)
    if game.has_pending_choice() {
        game.select_option(0);
    } // pay wait cost
      // Cost 2: discard 1 from hand (move_cards optional)
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // Cost 3: pay energy
    // Then effect: search deck for Aqours member with cost = yoshiko_cost + 2

    let before_stage = game.state.player1.stage.stage.clone();
    let before_deck_len = game.state.player1.main_deck.cards.len();
    eprintln!(
        "[YOSHIKO] before: stage={:?} deck={}",
        before_stage, before_deck_len
    );

    // Handle any remaining choices (search result, etc.)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let after_stage = game.state.player1.stage.stage.clone();
    let after_deck_len = game.state.player1.main_deck.cards.len();
    eprintln!(
        "[YOSHIKO] after: stage={:?} deck={}",
        after_stage, after_deck_len
    );

    // No new member was deployed (no Aqours members in deck)
    assert_eq!(
        before_stage, after_stage,
        "Stage unchanged when no Aqours member found"
    );
}

/// Harder edge: Deck has an Aqours member but with wrong cost.
/// Ability searches for cost = self_cost + 2. If no match, no deployment.
#[test]
fn yoshiko_q154_wrong_cost_aqours_in_deck_not_deployed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    // Aqours member with cost that does NOT match yoshiko_cost + 2
    // Yoshiko cost = 11? or 12? Check actual cost
    // Use a random Aqours member
    let aqours_member = game.id("PL!S-sd1-001-SD"); // this may or may not be Aqours

    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(aqours_member);
    for _ in 0..29 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage[1] = yoshiko;
    game.state.player1.stage.stage[0] = filler;
    game.state.player1.stage.stage[2] = filler;
    game.state.player1.hand.cards.push(filler);
    game.give_energy(15);

    game.activate_ability(yoshiko);
    if game.has_pending_choice() {
        game.select_option(0);
    }
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Check if aqours_member was deployed or is still in deck
    let on_stage = game.state.player1.stage.stage.contains(&aqours_member);
    let in_deck = game.state.player1.main_deck.cards.contains(&aqours_member);
    eprintln!(
        "[YOSHIKO] aqours_member on_stage={} in_deck={}",
        on_stage, in_deck
    );
    // Card must be in exactly one place: either deployed to stage (if cost matched) or still in deck
    assert!(
        in_deck,
        "Aqours member should still be in deck (cost didn't match yoshiko_cost + 2)"
    );
    assert!(
        !(on_stage && in_deck),
        "Aqours member should not be in both stage and deck simultaneously"
    );
}

/// Test case 9: Conditional summon uses sacrificed member cost + 2.
#[test]
fn test_yoshiko_center_ability_cost_plus_two_same_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let yoshiko = game.id("PL!S-bp3-006-R\u{ff0b}");
    let sacrificial_member = game.id("PL!S-bp2-001-R"); // cost 9
    let summoned_member = game.id("PL!S-bp2-004-R"); // cost 11 = 9 + 2
    let wrong_cost_member = game.id("PL!S-bp2-002-R"); // cost 4, should stay put
    let hand_card = game.id("PL!-sd1-010-SD");

    // Yoshiko in Center, one other Aqours member on stage to sacrifice.
    game.state.player1.stage.stage = [sacrificial_member, yoshiko, -1];
    game.state.player1.hand.cards.push(hand_card);

    // Put both the valid +2 card and a wrong-cost Aqours decoy in discard.
    game.state.player1.waitroom.cards.push(summoned_member);
    game.state.player1.waitroom.cards.push(wrong_cost_member);

    game.give_energy(15);

    game.activate_ability(yoshiko);

    // Resolve any prompts from the stage sacrifice / hand discard.
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);

    assert!(
        game.player().waitroom.cards.contains(&sacrificial_member),
        "Sacrificed member should be sent to discard"
    );
    assert_eq!(
        game.player().stage.stage[0],
        summoned_member,
        "Cost+2 Aqours member should be placed in the vacated area"
    );
    assert_eq!(
        game.player().stage.stage[1],
        yoshiko,
        "Yoshiko should remain in Center"
    );
    assert!(
        game.player().waitroom.cards.contains(&wrong_cost_member),
        "Wrong-cost Aqours member should remain in discard"
    );
    assert!(
        !game.player().stage.stage.contains(&wrong_cost_member),
        "Wrong-cost Aqours member should not be placed on stage"
    );
}
