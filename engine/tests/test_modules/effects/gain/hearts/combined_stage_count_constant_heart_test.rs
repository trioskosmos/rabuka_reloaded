use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn sp_pr_022_pr_constant_hearts_at_six_combined_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-PR-022-PR");
    let f1 = game.new_id(FILLER);
    let f2 = game.new_id(FILLER);
    game.state.player1.stage.stage = [me, f1, f2];
    for i in 0..3usize {
        let m = game.new_id(FILLER);
        game.state.player2.stage.stage[i] = m;
    }

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    const H03: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart03;
    assert!(
        game.state.mods.get_heart_modifier(me, H02) > 0,
        "6 combined members -> heart02"
    );
    assert!(
        game.state.mods.get_heart_modifier(me, H03) > 0,
        "6 combined members -> heart03"
    );
}

#[test]
fn s_pr_042_pr_constant_six_combined_staged_members_gains_heart02_and_heart04() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!S-PR-042-PR");
    let m1 = game.new_id(FILLER);
    let m2 = game.new_id(FILLER);
    game.state.player1.stage.stage = [me, m1, m2];
    // 3 more for p2 => 6 total including me.
    for i in 0..3usize {
        let m = game.new_id(FILLER);
        game.state.player2.stage.stage[i] = m;
    }

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    const H04: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart04;
    let h02 = game.state.mods.get_heart_modifier(me, H02);
    let h04 = game.state.mods.get_heart_modifier(me, H04);
    if h02 == 0 || h04 == 0 {
        panic!(
            "heart02={} heart04={}\nstaged p1={:?} p2={:?}\nstatuses={:#?}",
            h02,
            h04,
            game.state.player1.stage.stage,
            game.state.player2.stage.stage,
            game.state.constant_ability_statuses
        );
    }
    assert!(h02 > 0, "6 staged members -> heart02 granted");
    assert!(h04 > 0, "6 staged members -> heart04 granted");
}
