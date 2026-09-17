use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn opponent_surplus_gates_dynamic_live_total_bonus_pl_s_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R");
    game.add_to_stage(MemberArea::Center, mari);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "no opponent members → no surplus → no bonus"
    );

    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "opponent surplus 7 ≥ 2 → our live total +1"
    );

    game.state.player2.stage.stage[0] = -1;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "bonus removed once opponent surplus drops below 2"
    );
}

#[test]
fn opponent_surplus_exactly_two_grants_live_total_bonus_pl_s_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R");
    game.add_to_stage(MemberArea::Center, mari);

    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;
    let need5_live = game.id("PL!-sd1-020-SD");
    game.state.player2.live_card_zone.cards.push(need5_live);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "surplus exactly 2 (7 hearts − 5 needed) → bonus applies"
    );
}

#[test]
fn opponent_hearts_fully_consumed_by_need_grant_no_live_total_bonus_pl_s_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R");
    game.add_to_stage(MemberArea::Center, mari);

    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;
    let need7_live = game.id("PL!S-PR-023-PR");
    game.state.player2.live_card_zone.cards.push(need7_live);

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 0,
        "surplus 0 (7 hearts − 7 needed) < 2 → no bonus"
    );
}

#[test]
fn opponent_surplus_bonus_targets_live_total_not_card_pl_s_bp5_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp5-008-R");
    game.add_to_stage(MemberArea::Center, mari);
    let opp_member = game.new_id("PL!S-sd1-001-SD");
    game.state.player2.stage.stage[0] = opp_member;

    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 1,
        "live-total accumulator holds the +1"
    );
    assert_eq!(
        game.state.mods.get_score_modifier(mari),
        0,
        "ライブの合計スコア targets the TOTAL — no per-card modifier"
    );
}
