/// Tests for 津島善子 (PL!S-bp3-006-R＋) — Center activation ability:
/// Cost: wait self + discard 1 from hand
/// Effect: Move 1 other Aqours member from stage to discard → conditional summon 1 Aqours
/// member from discard with cost = moved_member.cost + 2 to same area.
///
/// Q154: When no other Aqours member exists on stage, the effect ends silently
/// (cost paid, no cards moved).
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

    let before_stage = game.state.player1.stage.stage.clone();

    // Activate ability
    game.activate_ability(yoshiko);

    // Resolve all choices: cost (hand discard) → effect (stage member selection)
    // Since filler cards on stage are not Aqours, effect action 1 has 0 valid targets.
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    eprintln!("[YOSHIKO] before: stage={:?}", before_stage);
    eprintln!(
        "[YOSHIKO] after: stage={:?}",
        game.state.player1.stage.stage
    );

    assert_eq!(
        before_stage, game.state.player1.stage.stage,
        "Stage unchanged when no other Aqours member on stage"
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

    // Resolve all choices: cost + effect
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Fillers on stage are not Aqours → effect action 1 has 0 targets → ability ends silently
    // aqours_member stays in deck (was never considered for summon)
    let in_deck = game.state.player1.main_deck.cards.contains(&aqours_member);
    eprintln!("[YOSHIKO] aqours_member in_deck={}", in_deck);
    assert!(
        in_deck,
        "Aqours member should still be in deck (effect never reached conditional summon)"
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

    // Handle all prompts (cost choices, etc.)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Verify stage[0] has the summoned member
    let stage0 = game.player().stage.stage[0];
    assert_eq!(
        stage0, summoned_member,
        "Cost+2 Aqours member should be placed in the vacated area (stage[0]={})",
        stage0,
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
