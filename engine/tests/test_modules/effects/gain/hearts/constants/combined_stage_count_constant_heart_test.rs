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
    // 「自分と相手のステージにメンバーが合計6人いるかぎり、{{heart_02}}{{heart_03}}を得る」
    // — ONE of each colour, two single icons side by side. `> 0` accepted 2 or 3, so a modifier
    // applied twice was invisible. Each colour is now pinned to exactly 1.
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H02),
        1,
        "{{heart_02}} is printed once, so exactly +1 (got {})",
        game.state.mods.get_heart_modifier(me, H02)
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H03),
        1,
        "{{heart_03}} is printed once, so exactly +1 (got {})",
        game.state.mods.get_heart_modifier(me, H03)
    );
}

/// The bound: 合計**6人**いるかぎり — and five is never staged.
///
/// Both tests in this file stage 3 + 3 = exactly 6, so they sit ON the bound and can only
/// prove the inclusive side. A `>= 5` reading, or a sum that counts the acting member twice,
/// passes both. Five members is the only value that separates them.
///
/// 2 on p1 + 3 on p2 = 5.
#[test]
fn sp_pr_022_pr_five_combined_members_is_one_short_of_the_bound() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-PR-022-PR");
    let f1 = game.new_id(FILLER);
    let f2 = game.new_id(FILLER);
    // TWO of ours (me + f1), so 2 + 3 = 5. f2 is deliberately left off the stage.
    game.state.player1.stage.stage = [me, f1, -1];
    let _ = f2;
    for i in 0..3usize {
        let m = game.new_id(FILLER);
        game.state.player2.stage.stage[i] = m;
    }

    game.state.recalculate_constants();

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    const H03: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart03;
    let total = (0..3).filter(|i| game.state.player1.stage.stage[*i] != -1).count()
        + (0..3)
            .filter(|i| game.state.player2.stage.stage[*i] != -1)
            .count();
    assert_eq!(
        total, 5,
        "precondition: five members must be staged — one short of the printed 合計6人 bound"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H02),
        0,
        "合計6人いるかぎり — 5 is one short, so no heart02 (got {})",
        game.state.mods.get_heart_modifier(me, H02)
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(me, H03),
        0,
        "合計6人いるかぎり — 5 is one short, so no heart03 (got {})",
        game.state.mods.get_heart_modifier(me, H03)
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

#[test]
fn karin_exact_six_members_grants_heart02_and_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id("PL!N-PR-027-PR");
    assert_eq!(game.db.get_card(karin).unwrap().card_no, "PL!N-PR-027-PR");

    let p1_left = game.new_id(FILLER);
    let p1_right = game.new_id(FILLER);
    let p2_left = game.new_id(FILLER);
    let p2_center = game.new_id(FILLER);
    game.state.player1.stage.stage = [karin, p1_left, p1_right];
    game.state.player2.stage.stage = [p2_left, p2_center, -1];

    const H01: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart01;
    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    const H05: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart05;

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(karin, H02), 0);
    assert_eq!(game.state.mods.get_heart_modifier(karin, H05), 0);

    let p2_right = game.new_id(FILLER);
    game.state.player2.stage.stage[2] = p2_right;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(karin, H02), 1);
    assert_eq!(game.state.mods.get_heart_modifier(karin, H05), 1);
    assert_eq!(game.state.mods.get_heart_modifier(karin, H01), 0);

    game.state.player2.stage.stage[2] = -1;
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(karin, H02), 0);
    assert_eq!(game.state.mods.get_heart_modifier(karin, H05), 0);
}

#[test]
fn karin_counts_six_physical_instances_of_the_same_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin1 = game.id("PL!N-PR-027-PR");
    let karin2 = game.new_id("PL!N-PR-027-PR");
    let karin3 = game.new_id("PL!N-PR-027-PR");
    let karin4 = game.new_id("PL!N-PR-027-PR");
    let karin5 = game.new_id("PL!N-PR-027-PR");
    let karin6 = game.new_id("PL!N-PR-027-PR");
    let members = [karin1, karin2, karin3, karin4, karin5, karin6];
    assert!(members.windows(2).all(|pair| pair[0] != pair[1]));

    game.state.player1.stage.stage = [members[0], members[1], members[2]];
    game.state.player2.stage.stage = [members[3], members[4], -1];

    const H02: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart02;
    const H05: rabuka_engine::card::HeartColor = rabuka_engine::card::HeartColor::Heart05;

    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(karin1, H02), 0);
    assert_eq!(game.state.mods.get_heart_modifier(karin1, H05), 0);

    game.state.player2.stage.stage[2] = members[5];
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_heart_modifier(karin1, H02), 1);
    assert_eq!(game.state.mods.get_heart_modifier(karin1, H05), 1);
}
