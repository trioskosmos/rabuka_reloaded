use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn s_pr_039_pr_constant_four_combined_success_cards_gains_two_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-039-PR");
    game.state.player1.stage.stage[0] = me;

    for _ in 0..2 {
        let a = game.new_id(FILLER);
        game.state.player1.success_live_card_zone.cards.push(a);
        let b = game.new_id(FILLER);
        game.state.player2.success_live_card_zone.cards.push(b);
    }
    // Combined = 4.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        2,
        "4 combined success-zone cards -> +2 blades"
    );
}

#[test]
fn s_pr_039_pr_constant_two_combined_success_cards_gains_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-039-PR");
    game.state.player1.stage.stage[0] = me;

    let a = game.new_id(FILLER);
    game.state.player1.success_live_card_zone.cards.push(a);
    let b = game.new_id(FILLER);
    game.state.player2.success_live_card_zone.cards.push(b);
    // Combined = 2 < 4.

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(me),
        0,
        "only 2 combined success-zone cards -> no blades"
    );
}

#[test]
fn combined_success_count_threshold_grants_blades_pl_s_pb1_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-pb1-009-R");
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!SP-bp1-027-L");
    game.add_to_stage(MemberArea::Center, ruby);

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(ruby), 0, "empty zones");

    game.state.player1.success_live_card_zone.add_card(live_a);
    game.state
        .player2
        .success_live_card_zone
        .add_card(live_b);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "combined total 2 < 3 → no blade"
    );

    let live_c_opp = game.new_id("PL!-sd1-021-SD");
    game.state
        .player2
        .success_live_card_zone
        .add_card(live_c_opp);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "own+opponent combined 3 cards → blade+3"
    );
}

#[test]
fn combined_success_count_opponent_only_grants_blades_pl_s_pb1_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-pb1-009-R");
    game.add_to_stage(MemberArea::Center, ruby);
    for _ in 0..3 {
        let copy = if game
            .state
            .player2
            .success_live_card_zone
            .cards
            .is_empty()
        {
            game.id("PL!-sd1-019-SD")
        } else {
            game.new_id("PL!-sd1-019-SD")
        };
        game.state.player2.success_live_card_zone.add_card(copy);
    }
    assert_eq!(game.state.player1.success_live_card_zone.len(), 0);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "opponent-only 3 cards still satisfy 自分と相手の…合計3枚以上"
    );
}

#[test]
fn combined_success_count_includes_zero_score_and_removes_blades_dynamically_pl_s_pb1_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby = game.id("PL!S-pb1-009-R");
    let zero_live = game.id("PL!-bp3-019-L");
    game.add_to_stage(MemberArea::Center, ruby);
    game.state.player1.success_live_card_zone.add_card(zero_live);
    for _ in 1..3 {
        let copy = game.new_id("PL!-bp3-019-L");
        game.state.player2.success_live_card_zone.add_card(copy);
    }
    assert_eq!(
        game.state.player1.success_live_card_zone.len()
            + game.state.player2.success_live_card_zone.len(),
        3
    );

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        3,
        "three score-0 lives = 3 CARDS → blade+3 (count, not score)"
    );

    game.state.player2.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(ruby),
        0,
        "bonus removed when the combined count drops"
    );
}

#[test]
fn combined_success_count_blades_apply_independently_to_two_copies_pl_s_pb1_009_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ruby_a = game.id("PL!S-pb1-009-R");
    let ruby_b = game.new_id("PL!S-pb1-009-R");
    game.state.player1.stage.stage[0] = ruby_b;
    game.state.player1.stage.stage[1] = ruby_a;
    for _ in 0..3 {
        let copy = if game
            .state
            .player2
            .success_live_card_zone
            .cards
            .is_empty()
        {
            game.id("PL!-sd1-019-SD")
        } else {
            game.new_id("PL!-sd1-019-SD")
        };
        game.state.player2.success_live_card_zone.add_card(copy);
    }

    game.state.recalculate_constants();
    for &copy in &[ruby_a, ruby_b] {
        assert_eq!(
            game.state.mods.get_blade_modifier(copy),
            3,
            "each copy independently gets blade+3"
        );
    }
}
