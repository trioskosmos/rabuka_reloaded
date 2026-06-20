use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn place_under(game: &mut TestGame, area: MemberArea, card_id: i16) {
    game.state.player1.stage.place_under_card(area, card_id);
}

// Dispatch helper: queue a LiveSuccess ability and process it through the queue.
fn process_live_success_ability(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let live_success_ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .cloned()
        .expect("Card must have LiveSuccess ability");

    let ability_id = format!("{}_{}", card.card_no, live_success_ab.full_text);
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        game.state.player1.id.clone(),
        Some(card.card_no.clone()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    let pid = game.state.player1.id.clone();
    game.state.process_pending_auto_abilities(&pid);
}

// ================================================================
// PL!-bp6-003-R+ 南ことり — ab#1: ライブ成功時
// ================================================================

#[test]
fn kotori_deploy_to_empty_right() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse = game.id("PL!-sd1-005-SD"); // cost=2, μ's
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kotori, -1];
    place_under(&mut game, MemberArea::Center, muse);
    game.give_energy(3);

    process_live_success_ability(&mut game, kotori);

    assert!(
        game.has_pending_choice(),
        "should prompt to select under member"
    );
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.stage.stage[2], muse,
        "μ's member deployed to empty right"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
}

#[test]
fn kotori_deploy_to_left_when_right_full() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, kotori, filler];
    place_under(&mut game, MemberArea::Center, muse);
    game.give_energy(3);

    process_live_success_ability(&mut game, kotori);

    assert!(game.has_pending_choice());
    game.select_indices(&[0]);

    assert_eq!(game.state.player1.stage.stage[0], muse);
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        0
    );
}

#[test]
fn kotori_no_member_under_no_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kotori, -1];
    game.give_energy(3);

    process_live_success_ability(&mut game, kotori);

    assert!(!game.has_pending_choice(), "no choice when nothing under");
    assert_eq!(game.state.player1.stage.stage[2], -1);
}

#[test]
fn kotori_no_empty_slot_keeps_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kotori, filler];
    place_under(&mut game, MemberArea::Center, muse);
    game.give_energy(3);

    process_live_success_ability(&mut game, kotori);

    // No empty slot → no choice, member stays under
    assert!(!game.has_pending_choice());
    assert!(game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::Center)
        .contains(&muse));
}

#[test]
fn kotori_skip_optional_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kotori, -1];
    place_under(&mut game, MemberArea::Center, muse);
    game.give_energy(3);

    process_live_success_ability(&mut game, kotori);

    assert!(game.has_pending_choice());
    game.select_indices(&[]); // skip (allow_skip=true)

    assert_eq!(game.state.player1.stage.stage[2], -1);
    assert!(game
        .state
        .player1
        .stage
        .get_under_cards(MemberArea::Center)
        .contains(&muse));
}
