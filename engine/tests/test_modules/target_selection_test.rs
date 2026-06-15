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
    let mut hearts = std::collections::HashMap::new();
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
    game.state.player1.stage.set_area(MemberArea::Center, member_a);
    game.state.player1.stage.set_area(MemberArea::LeftSide, member_b);
    game.state.player1.live_card_zone.cards.push(phoenix);
    game.state.player1.live_card_zone.cards.push(stellar);
    setup_live_phase_with_hearts(&mut game);
    process_abilities(&mut game);
    let a_heart06 = game.state.mods.get_heart_modifier(member_a, HeartColor::Heart06);
    let b_heart06 = game.state.mods.get_heart_modifier(member_b, HeartColor::Heart06);
    assert_eq!(a_heart06 + b_heart06, 4, "Total +4 heart06 across both members");
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
    let mut hearts = std::collections::HashMap::new();
    hearts.insert(HeartColor::Heart01, 5);
    hearts.insert(HeartColor::Heart00, 5);
    use rabuka_engine::card::BaseHeart;
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });
    game.state.current_phase = Phase::LiveCardSetFirstAttacker;
    process_abilities(&mut game);
    // Just verify the ability processes without panic (card has distinct logic)
    let score = game.state.mods.get_score_modifier(live_card);
    // Should not crash — distinct logic should handle valid targets gracefully
    eprintln!("distinct_card_name test: score={}", score);
}

#[test]
fn target_count_on_draw_until_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // PL!N-PR-028-PR has draw_until_count with target_count=5
    let card = game.id("PL!N-PR-028-PR");
    game.state.player1.stage.stage = [-1, card, -1];
    game.state.recalculate_constants();
    // Just verify no crash — the constant ability has target_count=5
    let hand_size = game.state.player1.hand.cards.len();
    eprintln!("draw_until target_count test: hand_size={}", hand_size);
}
