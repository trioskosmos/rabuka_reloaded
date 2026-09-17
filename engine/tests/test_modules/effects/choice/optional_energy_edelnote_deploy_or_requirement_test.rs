use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::stats_pipeline::effective_need_heart;
use rabuka_engine::core::types::AbilityTrigger;

const LIVE: &str = "PL!HS-bp5-022-L";
const HIGH: &str = "PL!HS-bp5-007-R";
const LOW: &str = "PL!HS-sd1-007-SD";

fn setup(stage_card: &str) -> (TestGame, i16, i16, i16) {
    let mut game = TestGame::new(load_real_database());
    let live = game.id(LIVE);
    assert_eq!(game.db.get_card(live).unwrap().card_no, LIVE);
    let high = game.id(stage_card);
    let low = game.id(LOW);
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.stage.stage[1] = high;
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.waitroom.cards.push(low);
    game.give_energy(5);
    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");
    (game, live, high, low)
}

fn pay(game: &mut TestGame, accept: bool) {
    assert!(
        matches!(game.get_pending_choice(), Choice::SelectTarget { target, .. }
        if target == "pay_optional_cost:skip_optional_cost" || target == "conditional_optional")
    );
    game.select_option(if accept { 1 } else { 0 });
}

fn assert_requirement(game: &TestGame, live: i16, reduced: bool) {
    let card = game.db.get_card(live).unwrap();
    let mut expected = card.need_heart.clone().unwrap();
    if reduced {
        expected.hearts.insert(HeartColor::Heart06, 4);
    }
    let actual = effective_need_heart(
        card.need_heart.as_ref(),
        live,
        &game.state.mods.need_heart_modifiers,
    )
    .unwrap();
    assert_eq!(actual.hearts, expected.hearts);
}

#[test]
fn paid_energy_choice_reduces_only_own_heart06_requirement_743() {
    let (mut game, live, high, low) = setup(HIGH);
    pay(&mut game, true);
    assert!(game.has_pending_choice());
    game.select_option(1);
    game.drain_choices_strict(&[], &[]);
    assert_requirement(&game, live, true);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);
    assert_eq!(game.state.player1.stage.stage, [-1, high, -1]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[low]);
}

#[test]
fn paid_energy_choice_deploys_cost_four_edelnote_into_empty_area_743() {
    let (mut game, live, high, low) = setup(HIGH);
    pay(&mut game, true);
    assert!(game.has_pending_choice());
    game.select_option(0);
    assert!(matches!(
        game.get_pending_choice(),
        Choice::SelectPosition { .. }
    ));
    game.select_generated(0);
    game.drain_choices_strict(&[], &[]);
    assert_eq!(game.state.player1.stage.stage, [low, high, -1]);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);
    assert_requirement(&game, live, false);
}

#[test]
fn declined_energy_preserves_energy_stage_and_requirement_743() {
    let (mut game, live, high, low) = setup(HIGH);
    pay(&mut game, false);
    game.drain_choices_strict(&[], &[]);
    assert_eq!(game.state.player1.energy_zone.active_count(), 5);
    assert_eq!(game.state.player1.stage.stage, [-1, high, -1]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[low]);
    assert_requirement(&game, live, false);
}

#[test]
fn exact_cost_nine_edelnote_enables_requirement_choice_743() {
    let (mut game, live, high, low) = setup("PL!HS-bp5-016-N");
    assert_eq!(game.db.get_card(high).unwrap().cost, Some(9));
    pay(&mut game, true);
    game.select_option(1);
    game.drain_choices_strict(&[], &[]);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[low]);
    assert_requirement(&game, live, true);
}

#[test]
fn low_cost_edelnote_or_costly_wrong_group_cannot_enable_choice_743() {
    for stage_card in [LOW, "PL!S-bp5-006-R"] {
        let (mut game, live, high, low) = setup(stage_card);
        if game.has_pending_choice() {
            pay(&mut game, true);
        }
        game.drain_choices_strict(&[], &[]);
        assert_eq!(game.state.player1.stage.stage, [-1, high, -1]);
        assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[low]);
        assert_requirement(&game, live, false);
    }
}

#[test]
fn deploy_filter_excludes_cost_five_and_wrong_group_743() {
    let (mut game, live, high, low) = setup(HIGH);
    let wrong_group = game.new_id("PL!-sd1-010-SD");
    let too_costly = game.new_id("PL!HS-bp6-023-N");
    game.state.player1.waitroom.cards.insert(0, wrong_group);
    game.state.player1.waitroom.cards.insert(1, too_costly);
    pay(&mut game, true);
    game.select_option(0);
    assert!(matches!(
        game.get_pending_choice(),
        Choice::SelectPosition { .. }
    ));
    game.select_generated(0);
    game.drain_choices_strict(&[], &[]);
    assert_eq!(game.state.player1.stage.stage, [low, high, -1]);
    assert_eq!(
        game.state.player1.waitroom.cards.as_slice(),
        &[wrong_group, too_costly]
    );
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);
    assert_requirement(&game, live, false);
}

#[test]
fn full_stage_deploy_choice_cannot_replace_existing_members_743() {
    let (mut game, live, high, low) = setup(HIGH);
    let left = game.new_id("PL!-sd1-010-SD");
    let right = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [left, high, right];
    pay(&mut game, true);
    game.select_option(0);
    game.drain_choices_strict(&[], &[]);
    assert_eq!(game.state.player1.stage.stage, [left, high, right]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[low]);
    assert_eq!(game.state.player1.energy_zone.active_count(), 3);
    assert_requirement(&game, live, false);
}
