/// L0 gap coverage: additional Constant and LiveStart heart/blade abilities.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;

fn drain_skips(game: &mut TestGame) {
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

fn advance_live(game: &mut TestGame) {
    for _ in 0..7 {
        game.pass();
        drain_skips(game);
    }
}

/// PL!SP-bp5-011-R: 常時 left→heart02×3, center→heart03×3, right→heart05×3.
/// Tests all three position variants.
#[test]
fn sp_bp5_011_position_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp5-011-R");
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();

    // Left → heart02×3
    game.state.player1.stage.stage[0] = member;
    game.state.player1.stage.stage[1] = -1;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(member, HeartColor::Heart02),
        3,
        "left → +3 heart02"
    );

    // Center → heart03×3
    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.stage[1] = member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(member, HeartColor::Heart03),
        3,
        "center → +3 heart03"
    );

    // Right → heart05×3
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = member;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(member, HeartColor::Heart05),
        3,
        "right → +3 heart05"
    );
}

/// PL!N-bp7-007-R: 常時 このメンバーの下にあるエネルギーカード1枚につき、
/// heart02を得る。
#[test]
#[ignore = "card PL!N-bp7-007-R not found in DB"]
fn nico_bp7_007_per_under_energy_heart02() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!N-bp7-007-R");
    game.state.player1.stage.stage = [-1, member, -1];
    // Place 2 energy cards under the member
    let e1 = game.id("LL-E-001-SD");
    let e2 = game.id("LL-E-001-SD");
    if let Some(idx) = game.state.player1.stage.stage.iter().position(|&x| x == member) {
        game.state.player1.stage.under_cards[idx].push(e1);
        game.state.player1.stage.under_cards[idx].push(e2);
    }
    game.state.recalculate_constants();

    let h02 = game
        .state
        .mods
        .get_heart_modifier(member, HeartColor::Heart02);
    assert!(
        h02 >= 2,
        "2 under-member energies should grant >= +2 heart02"
    );
}

/// PL!S-bp7-005-R: 常時 メンバーカードが下に置かれている『Aqours』メンバーは、
/// ブレード+1。
/// TODO: needs investigation — under-card setup may not trigger correctly.
#[test]
#[ignore = "under-card blade needs investigation"]
fn sb7_005_aqours_under_card_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!S-bp7-005-R");
    game.state.player1.stage.stage = [-1, member, -1];
    // Place an Aqours member card under this member
    let under_card = game.id("PL!S-bp2-001-R");
    if let Some(idx) = game.state.player1.stage.stage.iter().position(|&x| x == member) {
        game.state.player1.stage.under_cards[idx].push(under_card);
    }
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(
        blade >= 1,
        "under-card Aqours member should grant >= +1 blade"
    );
}
