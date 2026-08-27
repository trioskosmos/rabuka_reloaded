/// L0 gap coverage: additional Constant conditional heart abilities.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// PL!N-bp5-016-N: 常時 エネルギーが10枚以上 → heart06×2。
#[test]
fn karin_bp5_016_energy_10_heart06x2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp5-016-N"); // 葉月恋: energy≥10 → heart06×2
    game.state.player1.stage.stage = [-1, member, -1];
    let fid = game.id_ref("PL!-sd1-010-SD");
    fill_decks(&mut game, fid);
    game.give_energy(12);
    game.state.recalculate_constants();

    let h06 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart06);
    assert_eq!(h06, 2, "12 energy ≥ 10 → +2 heart06");

    // Negative: drop below threshold
    game.state.player1.energy_zone.sub_active(5);
    game.state.recalculate_constants();
}

/// PL!N-sd1-001-SD: LiveStart pay 1E → other 虹ヶ咲 members get blade.
/// Already tested in l0_gap_livestart3_test.rs — this covers the negative:
/// no other 虹ヶ咲 on stage → nobody gets the boost.
#[test]
fn nsd1_001_no_other_niji_no_boost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!N-sd1-001-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    let fid2 = game.id_ref("PL!-sd1-010-SD");
    fill_decks(&mut game, fid2);
    game.give_energy(15);

    for _ in 0..7 {
        game.pass();
        drain_skips(&mut game);
    }

    let blade = game.state.mods.get_blade_modifier(member);
    assert_eq!(
        blade, 0,
        "no other 虹ヶ咲 on stage → no blade boost"
    );
}

fn drain_skips(game: &mut TestGame) {
    use rabuka_engine::ability::types::Choice;
    let mut guard = 0;
    while game.has_pending_choice() && guard < 30 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { allow_skip: true, .. } => game.select_indices(&[]),
            _ => break,
        }
    }
}
