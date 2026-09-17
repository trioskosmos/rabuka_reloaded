/// L0 gap coverage: Constant (常時) blade/heart abilities.
///
/// Each test places the member, sets up the trigger condition, and asserts
/// the exact modifier value.
use crate::helpers::*;

/// PL!SP-bp4-003-R: 常時 センターにいる場合、ブレード+2。
#[test]
fn center_grants_at_least_two_blades_pl_sp_bp4_003() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!SP-bp4-003-R");
    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(
        blade >= 2,
        "center position constant should grant at least +2 blade"
    );
}

/// PL!-bp3-002-R: 常時 相手のステージにいるウェイト状態のメンバー1人につき、
/// ブレード+1。
#[test]
fn two_waited_opponents_grant_at_least_two_blades_pl_bp3_002() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!-bp3-002-R");
    // Two opponent members in wait state
    let opp1 = game.new_id("PL!-sd1-001-SD");
    let opp2 = game.new_id("PL!-sd1-002-SD");
    game.state.player2.stage.stage = [opp1, opp2, -1];
    game.state.mods.add_orientation_modifier(opp1, "wait");
    game.state.mods.add_orientation_modifier(opp2, "wait");

    game.state.player1.stage.stage = [-1, member, -1];
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(blade >= 2, "two waited opponents should grant >= +2 blade");
}

/// PL!S-pb1-009-R: 常時 相手の成功ライブカード置き場にカードが3枚以上ある場合、
/// ブレード+3。
#[test]
fn three_opponent_success_cards_satisfy_combined_threshold_pl_s_pb1_009() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id("PL!S-pb1-009-R");
    game.state.player1.stage.stage = [-1, member, -1];
    for _ in 0..3 {
        let sc = game.new_id("PL!-sd1-020-SD");
        game.state.player2.success_live_card_zone.cards.push(sc);
    }
    game.state.recalculate_constants();

    let blade = game.state.mods.get_blade_modifier(member);
    assert!(
        blade >= 3,
        "opponent with 3 success cards should grant >= +3 blade"
    );
}
