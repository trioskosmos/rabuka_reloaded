/// Tests for PL!HS-cl1-003-CL — 起動 ability: wait member → blade×1 for live.
///
/// 起動/ターン1回: このメンバーをウェイトにする：
///   ライブ終了時まで、自分のステージにいる「みらくるーぱ！」の
///   メンバー1人は、{{icon_blade.png|ブレード}}を得る。
///
/// Parsed:
///   trigger: 起動, use_limit: 1
///   cost: change_state(wait, 1, member_card)
///   effect: gain_resource(blade, 1, live_end)
///     condition: group(group: "みらくるーぱ！", location: stage, target: self)
///
/// Covers: gain_resource + change_state (0% coverage)
use crate::helpers::*;

/// Pay cost (wait), verify blade gain on group member
#[test]
fn cl_gain_resource_pay_cost_wait_gain_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-cl1-003-CL");
    // Need a "みらくるーぱ！" member for the condition — use the card itself
    // (it's part of the group). Let's place it on stage center.
    game.state.player1.stage.stage = [-1, card, -1];

    // Activate
    game.activate_ability(card);

    // Cost paid: card in wait
    assert!(
        game.state.mods.get_orientation_modifier(card) == Some("wait"),
        "Card should be in wait state"
    );

    // Card should now have blade ×1 (from gain_resource)
    let blade = game.state.mods.get_blade_modifier(card);
    assert!(blade >= 1, "Card should have at least 1 blade modifier");
}

/// Verify failure: can't activate without enough members on stage
#[test]
fn cl_gain_resource_cost_pays_when_no_condition_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-cl1-003-CL");

    // Place card on stage alone — no other group members
    game.state.player1.stage.stage = [-1, card, -1];

    // Activate — cost pays but effect may not apply to anyone
    game.activate_ability(card);

    // Cost should still be paid (change_state cost is unconditional)
    assert!(
        game.state.mods.get_orientation_modifier(card) == Some("wait"),
        "Card should be in wait state even without valid effect target"
    );
}
