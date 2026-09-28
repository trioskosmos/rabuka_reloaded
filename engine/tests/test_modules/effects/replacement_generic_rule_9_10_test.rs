use crate::helpers::{load_real_database, TestGame};
use rabuka_engine::ability::enums::ActionType;
use rabuka_engine::ability::resolver::AbilityResolver;
use rabuka_engine::ability::types::{Choice, ChoiceResult};
use rabuka_engine::card::AbilityEffect;

fn draw(count: u8) -> AbilityEffect {
    AbilityEffect {
        action: ActionType::DrawCard,
        count: Some(count),
        ..Default::default()
    }
}

fn setup() -> (TestGame, AbilityResolver, i16) {
    let mut game = TestGame::new(load_real_database());
    let a = game.id("PL!-sd1-010-SD");
    let b = game.id("PL!HS-bp1-005-P");
    let c = game.id("PL!S-bp3-019-L");
    game.state.player1.main_deck.cards.extend([a, b, c, a, b, c]);
    let resolver = AbilityResolver::new(game.state.card_database.clone(), None);
    (game, resolver, a)
}

fn add_replacement(
    game: &mut TestGame,
    card_id: i16,
    alternate: AbilityEffect,
    optional: bool,
) {
    game.state.add_replacement_effect(
        card_id,
        game.state.player1.id.clone(),
        ActionType::DrawCard.to_str().to_string(),
        vec![alternate],
        optional,
    );
}

fn answer(resolver: &mut AbilityResolver, game: &mut TestGame, accept: bool) {
    assert!(matches!(
        resolver.awaiting.choice,
        Some(Choice::SelectTarget { ref target, .. }) if target == "apply_replacement"
    ));
    resolver
        .provide_choice_result(
            &mut game.state,
            ChoiceResult::TargetSelected {
                target: if accept { "0" } else { "1" }.to_string(),
            },
        )
        .unwrap();
}

#[test]
fn mandatory_replacement_executes_alternate_and_suppresses_original() {
    let (mut game, mut resolver, card_id) = setup();
    add_replacement(&mut game, card_id, draw(1), false);

    resolver
        .execute_effect(&mut game.state, &draw(2))
        .unwrap();

    assert_eq!(game.state.player1.hand.cards.len(), 1);
    assert!(game.state.replacement_effects[0].applied_this_event);
}

#[test]
fn declined_optional_replacement_executes_original() {
    let (mut game, mut resolver, card_id) = setup();
    add_replacement(&mut game, card_id, draw(1), true);

    assert_eq!(
        resolver
            .execute_effect(&mut game.state, &draw(2))
            .unwrap_err(),
        "Pending choice required: apply replacement effect"
    );
    answer(&mut resolver, &mut game, false);

    assert_eq!(game.state.player1.hand.cards.len(), 2);
    assert!(game.state.replacement_effects[0].applied_this_event);
}

#[test]
fn accepted_optional_replacement_executes_alternate_and_suppresses_original() {
    let (mut game, mut resolver, card_id) = setup();
    add_replacement(&mut game, card_id, draw(1), true);

    assert_eq!(
        resolver
            .execute_effect(&mut game.state, &draw(2))
            .unwrap_err(),
        "Pending choice required: apply replacement effect"
    );
    answer(&mut resolver, &mut game, true);

    assert_eq!(game.state.player1.hand.cards.len(), 1);
    assert!(game.state.replacement_effects[0].applied_this_event);
}

#[test]
fn each_registration_applies_at_most_once() {
    let (mut game, mut resolver, card_id) = setup();
    add_replacement(&mut game, card_id, draw(1), false);

    resolver
        .execute_effect(&mut game.state, &draw(2))
        .unwrap();

    assert_eq!(game.state.player1.hand.cards.len(), 1);
    assert!(game.state.replacement_effects[0].applied_this_event);
}

#[test]
fn two_registrations_on_one_card_are_independent() {
    let (mut game, mut resolver, card_id) = setup();
    add_replacement(&mut game, card_id, draw(1), false);
    add_replacement(&mut game, card_id, draw(2), false);

    resolver
        .execute_effect(&mut game.state, &draw(4))
        .unwrap();

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.replacement_effects.len(), 2);
    assert!(
        game
            .state
            .replacement_effects
            .iter()
            .all(|replacement| replacement.applied_this_event)
    );
}
