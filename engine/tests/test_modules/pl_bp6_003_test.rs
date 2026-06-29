use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn place_under(game: &mut TestGame, area: MemberArea, card_id: i16) {
    game.state.player1.stage.place_under_card(area, card_id);
}

fn seed_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

// Dispatch helper: queue a LiveStart ability and process it through the queue.
fn process_live_start_ability(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let live_start_ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .cloned()
        .expect("Card must have LiveStart ability");

    let ability_id = format!("{}_{}", card.card_no, live_start_ab.full_text);
    game.state.trigger_auto_ability(
        ability_id,
        rabuka_engine::core::types::AbilityTrigger::LiveStart,
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
// PL!-bp6-003-R+ 南ことり — ab#0: ライブ開始時
// Text: 手札にあるコスト2以下の『μ's』のメンバーカードを1枚公開し、
//       このメンバーの下に置いてもよい。そうした場合、好きなハートの
//       色を1つ指定する。ライブ終了時まで、そのハートを1つ得る。
// ================================================================

#[test]
fn kotori_live_start_place_one_card_get_one_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse = game.id("PL!-sd1-005-SD"); // cost=2, μ's member
    let filler_live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, kotori, -1];
    game.state.player1.hand.cards.push(muse);
    game.state.player1.hand.cards.push(filler_live);
    game.state.player1.hand.cards.push(filler);
    seed_deck(&mut game);
    game.give_energy(3);

    // Trigger LiveStart ability
    process_live_start_ability(&mut game, kotori);

    // Step 1: should prompt to select a μ's member from hand to put under
    assert!(
        game.has_pending_choice(),
        "LiveStart should prompt for card selection"
    );
    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } => {
            assert_eq!(*count, 1, "should ask for exactly 1 card");
            assert_eq!(zone, "hand", "should select from hand");
        }
        _ => panic!("Expected SelectCard choice"),
    }

    // Select the μ's member (index 0 in hand)
    game.select_indices(&[0]);

    // Step 2: should prompt to pick a heart color
    assert!(
        game.has_pending_choice(),
        "LiveStart should prompt for heart color"
    );
    let choice2 = game.get_pending_choice();
    match choice2 {
        rabuka_engine::ability::types::Choice::SelectHeartColor { count, .. } => {
            assert_eq!(*count, 1, "should ask for exactly 1 heart color");
        }
        _ => panic!("Expected SelectHeartColor choice"),
    }

    // Pick heart01
    game.select_option(0);

    // No more choices
    assert!(
        !game.has_pending_choice(),
        "no more choices after heart selected"
    );

    // Verify: member card is under center
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        1,
        "1 member card under center"
    );

    // Verify: heart01 modifier gained
    let heart = game
        .state
        .mods
        .get_heart_modifier(kotori, HeartColor::Heart01);
    assert_eq!(heart, 1, "heart01=1 gained");
}

#[test]
fn kotori_live_start_skip_does_not_gain_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse = game.id("PL!-sd1-005-SD"); // cost=2, μ's member
    let filler_live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, kotori, -1];
    game.state.player1.hand.cards.push(muse);
    game.state.player1.hand.cards.push(filler_live);
    game.state.player1.hand.cards.push(filler);
    seed_deck(&mut game);
    game.give_energy(3);

    process_live_start_ability(&mut game, kotori);

    // Should prompt to select card (optional, can skip)
    assert!(game.has_pending_choice());
    game.select_indices(&[]); // skip (empty selection = skip)

    // Should NOT prompt for heart color since no card was placed
    assert!(
        !game.has_pending_choice(),
        "no heart color choice when skipped"
    );

    // Verify: no cards under center
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
fn kotori_live_start_no_valid_target_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let filler_live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, kotori, -1];
    game.state.player1.hand.cards.push(filler_live);
    game.state.player1.hand.cards.push(filler);
    seed_deck(&mut game);
    game.give_energy(3);

    process_live_start_ability(&mut game, kotori);

    // No matching μ's member card in hand → should not prompt
    assert!(
        !game.has_pending_choice(),
        "no choice when no valid μ's member in hand"
    );
}

#[test]
fn kotori_live_start_only_muses_under_two_cost_selected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse_ok = game.id("PL!-sd1-005-SD"); // cost=2, μ's
    let too_expensive = game.id("PL!-sd1-014-SD"); // cost=4, μ's
    let filler_live = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage = [-1, kotori, -1];
    game.state.player1.hand.cards.push(muse_ok);
    game.state.player1.hand.cards.push(too_expensive);
    game.state.player1.hand.cards.push(filler_live);
    seed_deck(&mut game);
    game.give_energy(3);

    process_live_start_ability(&mut game, kotori);

    // Should prompt — only 1 valid card (muse_ok, cost=2 ≤2)
    assert!(game.has_pending_choice(), "should prompt with valid target");
    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => {
            assert_eq!(*count, 1, "should ask for exactly 1 card");
        }
        _ => panic!("Expected SelectCard choice"),
    }
}

#[test]
fn kotori_live_start_second_activation_asks_one_card_not_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse1 = game.id("PL!-sd1-005-SD"); // cost=2, μ's
    let muse2 = game.id("PL!-sd1-005-SD"); // another copy
    let filler_live = game.id("PL!-sd1-020-SD");

    game.state.player1.stage.stage = [-1, kotori, -1];
    game.state.player1.hand.cards.push(muse1);
    game.state.player1.hand.cards.push(muse2);
    game.state.player1.hand.cards.push(filler_live);

    // First activation
    process_live_start_ability(&mut game, kotori);

    // Select 1 card from hand to put under
    assert!(game.has_pending_choice(), "first activation: should prompt");
    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } => {
            assert_eq!(*count, 1, "first activation: should ask for exactly 1 card");
            assert_eq!(zone, "hand", "first activation: from hand");
        }
        _ => panic!("Expected SelectCard choice"),
    }
    game.select_indices(&[0]); // select the first muse

    // Pick a heart color
    assert!(
        game.has_pending_choice(),
        "first activation: should prompt for heart"
    );
    game.select_option(0); // heart01

    assert!(
        !game.has_pending_choice(),
        "first activation: no more choices"
    );

    // Verify: 1 card under center
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        1,
        "1 card under center after first activation"
    );

    // SECOND activation — this is the bug test
    process_live_start_ability(&mut game, kotori);

    assert!(
        game.has_pending_choice(),
        "second activation: should prompt"
    );
    let choice2 = game.get_pending_choice();
    match choice2 {
        rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } => {
            assert_eq!(
                *count, 1,
                "second activation: should ask for exactly 1 card (NOT 2)"
            );
            assert_eq!(zone, "hand", "second activation: from hand");
        }
        _ => panic!("Expected SelectCard choice"),
    }
}

// ================================================================
// PL!-bp6-003-R+ 南ことり — ab#1: ライブ成功時
// Existing tests from original file, plus edge case additions
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

#[test]
fn kotori_live_start_then_live_success_workflow() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse = game.id("PL!-sd1-005-SD"); // cost=2, μ's
    let filler_live = game.id("PL!-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, kotori, -1];
    game.state.player1.hand.cards.push(muse);
    game.state.player1.hand.cards.push(filler_live);
    game.state.player1.hand.cards.push(filler);
    seed_deck(&mut game);
    game.give_energy(3);

    // Step 1: LiveStart → place under
    process_live_start_ability(&mut game, kotori);
    assert!(game.has_pending_choice());
    // Check it's a SelectCard from hand with count=1
    let c = game.get_pending_choice();
    match c {
        rabuka_engine::ability::types::Choice::SelectCard { count, zone, .. } => {
            assert_eq!(*count, 1);
            assert_eq!(zone, "hand");
        }
        _ => panic!("Expected SelectCard"),
    }
    // Don't resolve yet -- we're checking interaction
}

#[test]
fn kotori_multiple_under_only_one_deployed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id("PL!-bp6-003-R+");
    let muse1 = game.id("PL!-sd1-005-SD");
    let muse2 = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, kotori, -1];
    place_under(&mut game, MemberArea::Center, muse1);
    place_under(&mut game, MemberArea::Center, muse2);
    game.give_energy(3);

    process_live_success_ability(&mut game, kotori);

    // Should prompt with count=1 (deploy only 1 card)
    assert!(game.has_pending_choice());
    let choice = game.get_pending_choice();
    match choice {
        rabuka_engine::ability::types::Choice::SelectCard { count, .. } => {
            assert_eq!(
                *count, 1,
                "should ask for exactly 1 card, even with multiple under"
            );
        }
        _ => panic!("Expected SelectCard"),
    }

    game.select_indices(&[0]);
    assert_eq!(game.state.player1.stage.stage[2], muse1);
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        1,
        "1 card remains under"
    );
}
