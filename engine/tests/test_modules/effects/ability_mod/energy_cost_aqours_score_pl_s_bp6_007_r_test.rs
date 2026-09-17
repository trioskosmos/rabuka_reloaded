use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_s_bp6_007_r_energy_cost_grants_aqours_constant_score_abilities() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanamaru = game.id("PL!S-bp6-007-R");
    game.add_to_stage(MemberArea::Center, hanamaru);
    let aqours_friend = game.id("PL!S-pb1-007-R");
    game.add_to_stage(MemberArea::LeftSide, aqours_friend);
    let outsider = game.id(FILLER);
    game.add_to_stage(MemberArea::RightSide, outsider);
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id("PL!-sd1-019-SD"));
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.new_id("PL!-sd1-019-SD"));
    game.give_energy(2);
    let hand_before = game.state.player1.hand.cards.len();
    fire_trigger(
        &mut game,
        hanamaru,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectTarget") => game.select_option(0),
            Some("SelectCard") => game.select_indices(&[0, 1]),
            _ => game.select_indices(&[]),
        }
    }
    assert!(game.state.player1.energy_zone.active_count() <= 2, "sanity");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "energy payment leaves hand alone"
    );
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus, 2,
        "up to TWO 『Aqours』 members each gain ライブの合計スコア+1 (μ's member excluded)"
    );
}
