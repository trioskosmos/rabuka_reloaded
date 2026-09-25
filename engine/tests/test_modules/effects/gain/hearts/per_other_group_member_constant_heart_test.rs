use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!-bp5-111-R: 常時 自分のステージにいる自分以外の『A-RISE』メンバー
/// 1人につき、heart05を得る。
#[test]
fn two_other_arise_members_grant_two_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-bp5-111-R");
    // Real A-RISE members (優木あんじゅ / 統堂英玲奈)
    let arise1 = game.id("PL!-bp5-222-R");
    let arise2 = game.id("PL!-bp5-333-R");
    game.state.player1.stage.stage = [arise1, member, arise2];
    game.state.recalculate_constants();

    let h05 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart05);
    assert_eq!(
        h05, 2,
        "two other A-RISE members → exactly +2 heart05"
    );
}

#[test]
fn two_copies_of_same_arise_member_grant_two_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!-bp5-111-R");
    let arise1 = game.new_id("PL!-bp5-333-R");
    let arise2 = game.new_id("PL!-bp5-333-R");
    game.state.player1.stage.stage = [member, arise1, arise2];
    game.state.recalculate_constants();

    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(member, HeartColor::Heart05),
        2
    );
}
