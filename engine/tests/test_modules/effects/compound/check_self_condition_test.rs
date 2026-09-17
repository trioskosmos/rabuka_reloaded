use crate::helpers::*;
use rabuka_engine::ability::condition::ConditionContext;
use rabuka_engine::card::{Condition, ConditionCommon};
use rabuka_engine::core::types::ArcStr;

/// Build a check_self location condition: "このカードが<location>にある場合".
fn self_location_condition(location: &str) -> Condition {
    Condition::Location {
        common: Box::new(ConditionCommon {
            location: Some(ArcStr::from(location)),
            target: Some(ArcStr::from("self")),
            count: Some(1),
            operator: Some(ArcStr::from(">=")),
            check_self: Some(true),
            ..Default::default()
        }),
        unit: None,
        group_reference: None,
        heart_type: None,
        state: None,
        sub_checks: None,
    }
}

/// check_self location condition tracks the ACTIVATING card's own presence:
/// true when this card is in the waitroom, false when it moved elsewhere —
/// even if OTHER matching cards remain in the zone.
#[test]
fn check_self_location_follows_the_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let self_card = game.id("PL!HS-sd1-008-SD");
    let other_card = game.id("PL!HS-sd1-010-SD");

    // Both cards in the waitroom; activating card is self_card.
    game.state.player1.waitroom.cards.push(self_card);
    game.state.player1.waitroom.cards.push(other_card);
    game.state.activating_card = Some(self_card);

    // Location variant
    let cond = self_location_condition("discard");
    let ctx = ConditionContext::new(&game.state);
    assert!(
        ctx.evaluate_condition(&cond),
        "check_self should pass while THIS card is in the waitroom"
    );

    // Regression guard: with a DIFFERENT card in the waitroom and this card
    // moved to hand, the condition must NOT match (pre-fix engine counted any
    // card).
    game.state
        .player1
        .waitroom
        .cards
        .retain(|id| *id != self_card);
    game.state.player1.hand.cards.push(self_card);
    let ctx2 = ConditionContext::new(&game.state);
    assert!(
        !ctx2.evaluate_condition(&cond),
        "check_self should fail when this card left, even though another card is in the waitroom"
    );
}

/// Same semantics through the comparison_condition container
/// ("このカードが手札にある場合" style gates).
#[test]
fn check_self_comparison_container_hand_presence() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let self_card = game.id("PL!HS-sd1-008-SD");

    let cond = Condition::Comparison {
        common: Box::new(ConditionCommon {
            location: Some(ArcStr::from("hand")),
            target: Some(ArcStr::from("self")),
            count: Some(1),
            operator: Some(ArcStr::from(">=")),
            check_self: Some(true),
            ..Default::default()
        }),
        values: None,
        cost_total: None,
        cost_total_operator: None,
        comparison_source: None,
        state: None,
        ability_filter: None,
    };

    game.state.player1.hand.cards.push(self_card);
    game.state.activating_card = Some(self_card);
    let ctx = ConditionContext::new(&game.state);
    assert!(
        ctx.evaluate_condition(&cond),
        "comparison_condition with check_self should pass while this card is in hand"
    );

    game.state
        .player1
        .waitroom
        .cards
        .push(game.id("PL!HS-sd1-010-SD"));
    game.state.player1.hand.cards.clear();
    let ctx2 = ConditionContext::new(&game.state);
    assert!(
        !ctx2.evaluate_condition(&cond),
        "comparison_condition with check_self should fail once this card leaves hand"
    );
}
