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

const CHEAP_MEMBER: &str = "PL!-sd1-010-SD"; // cost 4
const DEAR_MEMBER: &str = "PL!-sd1-014-SD"; // cost 9
const DEAREST_MEMBER: &str = "PL!SP-bp2-006-P"; // cost 10

/// Stage Emma with a cost-4 and a cost-9 member already in the waitroom, activate
/// her ability, pay the cost by discarding `discard_card`, and report what
/// happened: `(did the effect ask?, hand, waitroom)`.
///
/// The "did it ask" half is what the boundary tests below are about. The eligible
/// SET is the thing that changes between them, and a set of exactly one is
/// invisible in the resulting hand unless you also notice that nothing was offered.
fn emma_008_recover_from(discard_card: i16) -> (bool, Vec<i16>, Vec<i16>) {
    let db = load_real_database();
    let mut v = TestGame::new(db);

    let emma = v.id("PL!N-bp1-008-R");
    let cheap = v.id(CHEAP_MEMBER);
    let dear = v.id(DEAR_MEMBER);

    // The boundary is meaningless if the fixtures are not what this file thinks
    // they are, so the costs are read back from the database. `TestGame::id`
    // consumes a pool slot per call, hence reading them off the ids already taken
    // rather than looking them up again.
    let cost_of = |id: i16| v.db.get_card(id).and_then(|c| c.cost);
    assert_eq!(cost_of(cheap), Some(4), "{CHEAP_MEMBER} must cost 4");
    assert_eq!(cost_of(dear), Some(9), "{DEAR_MEMBER} must cost 9");

    v.state.player1.hand.cards.clear();
    v.state.player1.waitroom.cards.clear();
    v.state.player1.main_deck.cards.clear();

    v.state.player1.hand.cards.push(discard_card);
    v.state.player1.waitroom.cards.push(cheap);
    v.state.player1.waitroom.cards.push(dear);
    v.state.player1.stage.stage = [emma, -1, -1];
    let filler = v.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        v.state.player1.main_deck.cards.push(filler);
    }

    v.activate_ability(emma);
    assert_eq!(
        v.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the cost asks which member to discard"
    );
    v.select_indices(&[0]);

    let asked = v.has_pending_choice();
    if asked {
        assert_eq!(
            v.pending_choice_type().as_deref(),
            Some("SelectCard"),
            "the only thing left to ask about is which of the eligible cards"
        );
        v.select_indices(&[0]);
    }
    drain_auto(&mut v);

    (
        asked,
        v.state.player1.hand.cards.iter().copied().collect(),
        v.state.player1.waitroom.cards.iter().copied().collect(),
    )
}

/// Both waitroom cards are cheaper than the one the cost discarded, so both are
/// eligible and the engine must offer a choice between them.
#[test]
fn emma_008_both_cheaper_cards_are_offered_when_the_discard_is_the_dearest() {
    let db = load_real_database();
    let v = TestGame::new(db);
    let dearest = v.id(DEAREST_MEMBER);
    assert_eq!(
        v.db.get_card(dearest).and_then(|c| c.cost),
        Some(10),
        "{DEAREST_MEMBER} must cost 10"
    );

    let (asked, hand, waitroom) = emma_008_recover_from(dearest);

    assert!(
        asked,
        "4 < 10 and 9 < 10, so the eligible set has two cards and the engine has \
         to ask which one"
    );
    assert_eq!(
        hand.len(),
        1,
        "exactly one member comes back; hand is {hand:?}"
    );
    assert!(
        waitroom.contains(&dearest),
        "the discarded card stays in the waitroom; it is not < 10 either, but it \
         was named by the cost, not offered as a target"
    );
    assert_eq!(
        waitroom.len(),
        2,
        "3 went in (the two waitroom members + the cost card) and 1 came back out; \
         waitroom is {waitroom:?}"
    );
}

/// The same two waitroom cards, but the cost discards the cost-9 member. Only the
/// cost-4 card is strictly cheaper, so it resolves silently.
///
/// This is the case that pins WHERE the number comes from. Every other test in this
/// file leaves at most one eligible card in the waitroom, so they all agree with
/// each other: a bound hardcoded as "< 4", one read as "cheapest card in the
/// waitroom", and one read as "the card this cost discarded" give the same answer
/// on all of them. Here only the discarded card differs from the test above, and it
/// changes the answer.
///
/// It also pins the operator. 「コストの**低い**」 is strict, so the cost-9 card just
/// discarded is not a candidate for itself; a `<=` bound would offer it, turning a
/// silent resolution into a prompt.
#[test]
fn emma_008_only_the_strictly_cheaper_card_is_eligible() {
    let db = load_real_database();
    let v = TestGame::new(db);
    let dear = v.id(DEAR_MEMBER);

    let (asked, hand, waitroom) = emma_008_recover_from(dear);

    assert!(
        !asked,
        "only the cost-4 member is < 9, so there is nothing to choose between"
    );
    assert_eq!(
        hand.len(),
        1,
        "the cost-4 member comes back; hand is {hand:?}"
    );
    assert!(
        waitroom.contains(&dear),
        "the cost-9 card is not < 9, so it stays; waitroom is {waitroom:?}"
    );
    assert!(
        !hand.contains(&dear),
        "「コストの低い」 is strict — a `<=` bound would hand back the very card \
         this ability just discarded"
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
