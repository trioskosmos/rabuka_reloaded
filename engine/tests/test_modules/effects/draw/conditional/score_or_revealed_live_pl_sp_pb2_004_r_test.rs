use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn pl_sp_pb2_004_r_live_success_draw_score_or_revealed_live_and_neither() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-pb2-004-R");
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.add_to_stage(MemberArea::Center, sumire);
    let live6 = game.id("PL!SP-bp1-027-L");
    game.state.player1.live_card_zone.cards.push(live6);
    fire_trigger(
        &mut game,
        sumire,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "no condition met → no draw"
    );
    game.state.mods.add_score_modifier(live6, 2);
    fire_trigger(
        &mut game,
        sumire,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "元々のスコアより高いスコアのライブカード → draw 1"
    );
    game.state.mods.add_score_modifier(live6, -2);
    game.state.revealed_cards.push(game.id("PL!-sd1-019-SD"));
    fire_trigger(
        &mut game,
        sumire,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "revealed scored live → draw 1 via the OR branch"
    );
}
