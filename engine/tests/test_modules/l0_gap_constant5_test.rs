/// L0 gap coverage: additional Constant (常時) heart abilities.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!-pb1-002-R: 常時 相手のステージにいるウェイト状態のメンバー1人につき、
/// heart06を得る。
/// TODO: needs investigation - per-opponent-waited counting path.
#[test]
#[ignore = "per-opponent-waited heart06 needs investigation"]
fn pb1_002_per_opponent_waited_heart06() {
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

/// PL!SP-bp7-009-R: 常時 左サイドまたは右サイドにいる場合、heart02+1。
#[test]
fn sp_bp7_009_left_right_grants_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp7-009-R");

    // Left side
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.recalculate_constants();
    let h02_left = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02);
    assert!(h02_left >= 1, "left → +1 heart02");

    // Right side
    game.state.player1.stage.stage = [-1, -1, member];
    game.state.recalculate_constants();
    let h02_right = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02);
    assert!(h02_right >= 1, "right → +1 heart02");
}

/// PL!-bp5-111-R: 常時 自分のステージにいる自分以外の『A-RISE』メンバー
/// 1人につき、heart05を得る。
/// TODO: needs investigation - A-RISE group matching may need specific data.
#[test]
#[ignore = "A-RISE group matching needs investigation"]
fn bp5_111_per_other_arise_member_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-bp5-111-R");
    // A-RISE members on stage
    let arise1 = game.new_id("PL!-sd1-003-SD");
    let arise2 = game.new_id("PL!-sd1-004-SD");
    game.state.player1.stage.stage = [arise1, member, arise2];
    game.state.recalculate_constants();

    let h05 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart05);
    assert!(
        h05 >= 2,
        "two other A-RISE members should grant >= +2 heart05"
    );
}
