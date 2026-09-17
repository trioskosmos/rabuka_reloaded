use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_pr_020_pr_live_total_eight_grants_constant_score_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let honoka = game.id("PL!-PR-020-PR");
    game.add_to_stage(MemberArea::Center, honoka);
    game.state.player1.live_card_zone.cards.push(game.id("PL!SP-bp1-027-L"));
    game.state.player1.live_card_zone.cards.push(game.id("PL!-sd1-019-SD"));
    game.state.player1.live_card_zone.cards.push(game.new_id("PL!-sd1-019-SD"));
    let mut heart_map = HeartMap::new();
    heart_map.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts: heart_map });
    fire_trigger(&mut game, honoka, AbilityTrigger::LiveStart, "ライブ開始時");
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1, "score total ≥ 8 → gained 【常時】ライブの合計スコア+1");
}
