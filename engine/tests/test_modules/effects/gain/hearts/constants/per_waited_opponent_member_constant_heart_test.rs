use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!-pb1-002-R: 常時 相手のステージにいるウェイト状態のメンバー1人につき、
/// heart06を得る。
#[test]
fn one_waited_opponent_member_grants_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-pb1-002-R");
    let opp_waited = game.new_id("PL!-sd1-001-SD");
    game.state.player2.stage.stage = [opp_waited, -1, -1];
    game.state.mods.add_orientation_modifier(opp_waited, "wait");
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();

    let h06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert!(h06 >= 1, "one waited opponent → >= +1 heart06");
}
