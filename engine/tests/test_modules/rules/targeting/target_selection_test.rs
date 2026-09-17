use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::game_state::Phase;
use rabuka_engine::zones::MemberArea;

fn process_abilities(game: &mut TestGame) {
    let player_id = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_live_start_abilities(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);
    while game.state.has_pending_choice() {
        game.select_indices(&[0]);
        game.state.process_pending_auto_abilities(&player_id);
    }
}

fn setup_live_phase_with_hearts(game: &mut TestGame) {
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    let mut hearts = rabuka_engine::card::HeartMap::new();
    hearts.insert(HeartColor::Heart01, 7);
    hearts.insert(HeartColor::Heart02, 2);
    hearts.insert(HeartColor::Heart06, 6);
    hearts.insert(HeartColor::Heart00, 10);
    use rabuka_engine::card::BaseHeart;
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
}

#[test]
fn target_count_1_gain_resource_chooses_one_of_many() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member_a = game.id("PL!N-PR-003-PR");
    let member_b = game.id("PL!N-PR-005-PR");
    let phoenix = game.id("PL!N-pb1-038-L");
    let stellar = game.id("PL!N-pb1-039-L");
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state
        .player1
        .stage
        .set_area(MemberArea::Center, member_a);
    game.state
        .player1
        .stage
        .set_area(MemberArea::LeftSide, member_b);
    game.state.player1.live_card_zone.cards.push(phoenix);
    game.state.player1.live_card_zone.cards.push(stellar);
    setup_live_phase_with_hearts(&mut game);
    process_abilities(&mut game);
    let a_heart06 = game
        .state
        .mods
        .get_heart_modifier(member_a, HeartColor::Heart06);
    let b_heart06 = game
        .state
        .mods
        .get_heart_modifier(member_b, HeartColor::Heart06);
    assert_eq!(
        a_heart06 + b_heart06,
        4,
        "Total +4 heart06 across both members"
    );
    assert!(
        a_heart06 == 4 || b_heart06 == 4,
        "Exactly one member got the full +4 heart06 buff"
    );
}

#[test]
fn distinct_card_name_prevents_same_card_twice() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // Use a card that has a LiveStart ability with distinct=card_name
    // modify_required_hearts with distinct=card_name
    let live_card = game.id("PL!SP-bp1-026-L");
    game.state.player1.live_card_zone.cards.push(live_card);
    let mut hearts = rabuka_engine::card::HeartMap::new();
    hearts.insert(HeartColor::Heart01, 5);
    hearts.insert(HeartColor::Heart00, 5);
    use rabuka_engine::card::BaseHeart;
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    process_abilities(&mut game);
    // No members on stage/waitroom → the 5-distinct-Liella condition is unmet,
    // so the required-hearts replacement must NOT apply.
    let h02 = game
        .state
        .mods
        .get_need_heart_modifier(live_card, rabuka_engine::card::HeartColor::Heart02);
    assert_eq!(
        h02, 0,
        "condition unmet: required hearts must be unchanged, got modifier {h02}"
    );
}

#[test]
fn target_count_on_draw_until_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // PL!N-PR-028-PR: 登場, optional cost (discard 2 from hand) → draw to 5.
    let card = game.id("PL!N-PR-028-PR");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(30);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.play_to_stage(card, rabuka_engine::zones::MemberArea::Center);
    // Pay the optional cost with the two remaining hand cards.
    assert!(
        game.has_pending_choice(),
        "optional discard-2 cost prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (hand, count=2, allow_skip)"
    );
    game.select_indices(&[0, 1]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        5,
        "draw_until_count must fill the hand to target_count=5"
    );
}
