use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

#[test]
fn success_score_sum_threshold_and_boundary_grants_hearts_pl_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let live3 = game.id("PL!-sd1-021-SD");
    game.add_to_stage(MemberArea::Center, hanayo);

    game.state.player1.success_live_card_zone.add_card(live3);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "score total 3 < 6 竊・no heart03"
    );

    let live3b = game.new_id("PL!-sd1-021-SD");
    game.state.player1.success_live_card_zone.add_card(live3b);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        2,
        "score total exactly 6 竊・+2 heart03"
    );

    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "as_long_as bonus removed once total drops below 6"
    );

    game.state.player1.success_live_card_zone.add_card(live3);
    game.state
        .player1
        .success_live_card_zone
        .add_card(live3b);
    game.state.recalculate_constants();
    let hearts = game.state.player1.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &game.state.mods.heart_override,
        &game.state.mods.heart_modifiers,
        &game.state.mods.heart_copy,
    );
    let with_bonus = hearts.hearts[&HeartColor::Heart03];

    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    let hearts_off = game.state.player1.calculate_stage_hearts(
        &game.db,
        &game.state.mods.heart_color_multiplier,
        &game.state.mods.heart_override,
        &game.state.mods.heart_modifiers,
        &game.state.mods.heart_copy,
    );
    assert_eq!(
        with_bonus - hearts_off.hearts[&HeartColor::Heart03],
        2,
        "stage-aggregate heart03 must grow by exactly the constant's 2"
    );
}

#[test]
fn success_score_sum_grants_fixed_exact_color_hearts_pl_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let big_live = game.id("PL!S-pb1-023-L");
    game.add_to_stage(MemberArea::Center, hanayo);
    game.state
        .player1
        .success_live_card_zone
        .add_card(big_live);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        2,
        "total 9 竕･ 6 still grants exactly +2 (text says 縺､蠕励ｋ, fixed)"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart01),
        0,
        "no collateral heart01"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(hanayo),
        0,
        "no collateral blade either"
    );
}

#[test]
fn success_score_sum_ignores_opponent_zone_pl_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let big_live_p2 = game.new_id("PL!S-pb1-023-L");
    game.add_to_stage(MemberArea::Center, hanayo);
    game.state
        .player2
        .success_live_card_zone
        .add_card(big_live_p2);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "opponent zone total 9 must NOT count for 閾ｪ蛻・・窶ｦ"
    );
}

#[test]
fn success_score_sum_not_card_count_gates_hearts_pl_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo = game.id("PL!-bp5-008-R");
    let zero_live = game.id("PL!-bp3-019-L");
    game.add_to_stage(MemberArea::Center, hanayo);
    for _ in 0..6 {
        let copy = if game.state.player1.success_live_card_zone.cards.is_empty() {
            zero_live
        } else {
            game.new_id("PL!-bp3-019-L")
        };
        game.state.player1.success_live_card_zone.add_card(copy);
    }
    assert_eq!(game.state.player1.success_live_card_zone.len(), 6);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(hanayo, HeartColor::Heart03),
        0,
        "six score-0 lives = score sum 0 < 6 竊・no hearts (aggregate=score)"
    );
}

#[test]
fn success_score_sum_hearts_require_stage_presence_pl_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live6 = game.id("PL!SP-bp1-027-L");
    game.state.player1.success_live_card_zone.add_card(live6);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(game.id("PL!-bp5-008-R"), HeartColor::Heart03),
        0,
        "constant only works while she is on stage"
    );
}

#[test]
fn success_score_sum_hearts_apply_independently_to_two_copies_pl_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanayo_a = game.id("PL!-bp5-008-R");
    let hanayo_b = game.new_id("PL!-bp5-008-R");
    let live6 = game.id("PL!SP-bp1-027-L");
    game.state.player1.stage.stage[0] = hanayo_b;
    game.state.player1.stage.stage[1] = hanayo_a;
    game.state.player1.success_live_card_zone.add_card(live6);

    game.state.recalculate_constants();
    for &copy in &[hanayo_a, hanayo_b] {
        assert_eq!(
            game.state.mods.get_heart_modifier(copy, HeartColor::Heart03),
            2,
            "each copy independently gets +2 heart03"
        );
    }
}
