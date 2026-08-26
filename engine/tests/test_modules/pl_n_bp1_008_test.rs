use crate::helpers::*;

fn drain_auto(v: &mut TestGame) {
    while v.has_pending_choice() {
        match v.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => v.select_indices(&[0]),
            _ => v.select_indices(&[]),
        }
    }
}

/// PL!N-bp1-008 (Emma): cost: discard 1 member from hand;
/// effect: retrieve 1 member from waitroom with cost < discarded card's cost
#[test]
fn emma_008_discard_cost_9_retrieve_cost_4() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let emma = v.id("PL!N-bp1-008-R");
    let discard_card = v.id("PL!-sd1-014-SD"); // cost 9, no ability
    let lower_card = v.id("PL!-sd1-010-SD"); // cost 4, no ability
    let higher_card = v.id("PL!-sd1-017-SD"); // cost 9, no ability

    v.state.player1.hand.cards.clear();
    v.state.player1.waitroom.cards.clear();
    v.state.player1.main_deck.cards.clear();

    v.state.player1.hand.cards.push(discard_card);
    v.state.player1.waitroom.cards.push(higher_card); // cost 9, NOT < 9 → ineligible
    v.state.player1.waitroom.cards.push(lower_card); // cost 4 < 9 → eligible
    v.state.player1.stage.stage = [emma, -1, -1];
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }

    v.activate_ability(emma);

    // Cost: select 1 member from hand to discard (mandatory SelectCard, allow_skip=false)
    assert!(v.has_pending_choice(), "hand discard cost prompt expected");
    assert_eq!(
        v.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "cost should be a SelectCard prompt"
    );
    v.select_indices(&[0]);

    // Effect: single eligible candidate (cost 4 < 9) auto-resolves — no prompt.
    assert!(
        !v.has_pending_choice(),
        "single-candidate retrieve should auto-resolve without prompting"
    );

    drain_auto(&mut v);

    assert!(
        v.state.player1.hand.cards.contains(&lower_card),
        "lower_card (cost 4) should be in hand"
    );
    assert!(
        v.state.player1.waitroom.cards.contains(&discard_card),
        "discard_card should be in waitroom"
    );
    assert!(
        v.state.player1.waitroom.cards.contains(&higher_card),
        "higher_card (cost 9) should still be in waitroom"
    );
}

/// Same ability, discarded card cost 4, waitroom has cost 9 → no eligible target
#[test]
fn emma_008_discard_low_cost_no_eligible_target() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let emma = v.id("PL!N-bp1-008-R");
    let discard_card = v.id("PL!-sd1-010-SD"); // cost 4, no ability
    let higher_card = v.id("PL!-sd1-014-SD"); // cost 9, no ability

    v.state.player1.hand.cards.clear();
    v.state.player1.waitroom.cards.clear();
    v.state.player1.main_deck.cards.clear();

    v.state.player1.hand.cards.push(discard_card);
    v.state.player1.waitroom.cards.push(higher_card); // cost 9, NOT < 4 → ineligible
    v.state.player1.stage.stage = [emma, -1, -1];
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }

    v.activate_ability(emma);

    // Cost: select 1 member from hand to discard (mandatory SelectCard, allow_skip=false)
    assert!(v.has_pending_choice(), "hand discard cost prompt expected");
    assert_eq!(
        v.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "cost should be a SelectCard prompt"
    );
    v.select_indices(&[0]);

    // No eligible target (cost 9 NOT < 4): retrieve auto-skips without prompting.
    assert!(
        !v.has_pending_choice(),
        "zero-candidate retrieve should auto-skip without prompting"
    );

    drain_auto(&mut v);

    // No eligible target, so no card retrieved
    assert!(
        v.state.player1.waitroom.cards.contains(&discard_card),
        "discard_card should be in waitroom"
    );
    assert!(
        v.state.player1.waitroom.cards.contains(&higher_card),
        "higher_card should still be in waitroom"
    );
    assert!(
        v.state.player1.hand.cards.is_empty(),
        "Hand should be empty (nothing retrieved)"
    );
}

/// Multiple eligible targets in waitroom → should prompt for choice
#[test]
fn emma_008_multiple_eligible_targets_prompts_choice() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let emma = v.id("PL!N-bp1-008-R");
    let discard_card = v.id("PL!-sd1-014-SD"); // cost 9
    let target_a = v.id("PL!-sd1-010-SD"); // cost 4, no ability
    let target_b = v.id("PL!-sd1-013-SD"); // cost 4, no ability

    v.state.player1.hand.cards.clear();
    v.state.player1.waitroom.cards.clear();
    v.state.player1.main_deck.cards.clear();

    v.state.player1.hand.cards.push(discard_card);
    v.state.player1.waitroom.cards.push(target_a); // cost 4 < 9 → eligible
    v.state.player1.waitroom.cards.push(target_b); // cost 4 < 9 → eligible
    v.state.player1.stage.stage = [emma, -1, -1];
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }

    v.activate_ability(emma);

    // Cost: select 1 from hand (mandatory SelectCard, allow_skip=false)
    assert!(v.has_pending_choice(), "hand discard cost prompt expected");
    assert_eq!(
        v.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "cost should be a SelectCard prompt"
    );
    v.select_indices(&[0]);

    // Effect: 2 eligible cards in waitroom → engine prompts SelectCard zone=discard.
    assert!(
        v.has_pending_choice(),
        "retrieve prompt expected with multiple eligible targets"
    );
    assert_eq!(
        v.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "effect should be a SelectCard prompt"
    );
    // Select target_a (first eligible)
    v.select_indices(&[0]);

    drain_auto(&mut v);

    assert!(
        v.state.player1.hand.cards.contains(&target_a),
        "target_a should be in hand"
    );
    assert!(
        v.state.player1.waitroom.cards.contains(&discard_card),
        "discard_card in waitroom"
    );
    // target_b should still be in waitroom (only 1 retrieved)
    assert!(
        v.state.player1.waitroom.cards.contains(&target_b),
        "target_b should still be in waitroom"
    );
}

/// Discarding a card with cost 9, waitroom has equal cost (9) → not retrievable
/// (already verified by test 1, but testing only-equal scenario)
#[test]
fn emma_008_only_equal_cost_not_retrieved() {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let emma = v.id("PL!N-bp1-008-R");
    let discard_card = v.id("PL!-sd1-014-SD"); // cost 9
    let equal_card = v.id("PL!-sd1-017-SD"); // cost 9, no ability

    v.state.player1.hand.cards.clear();
    v.state.player1.waitroom.cards.clear();
    v.state.player1.main_deck.cards.clear();

    v.state.player1.hand.cards.push(discard_card);
    v.state.player1.waitroom.cards.push(equal_card); // cost 9, NOT < 9 → ineligible
    v.state.player1.stage.stage = [emma, -1, -1];
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(v.id("PL!-sd1-010-SD"));
    }

    v.activate_ability(emma);

    // Cost: select 1 member from hand to discard (mandatory SelectCard, allow_skip=false)
    assert!(v.has_pending_choice(), "hand discard cost prompt expected");
    assert_eq!(
        v.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "cost should be a SelectCard prompt"
    );
    v.select_indices(&[0]);

    // Equal-cost card is not retrievable (cost 9 NOT < 9): retrieve auto-skips, no prompt.
    assert!(
        !v.has_pending_choice(),
        "zero-candidate retrieve should auto-skip without prompting"
    );

    drain_auto(&mut v);

    assert!(
        v.state.player1.waitroom.cards.contains(&discard_card),
        "discard_card in waitroom"
    );
    assert!(
        v.state.player1.waitroom.cards.contains(&equal_card),
        "equal cost card still in waitroom"
    );
    assert!(
        v.state.player1.hand.cards.is_empty(),
        "Hand should be empty"
    );
}
