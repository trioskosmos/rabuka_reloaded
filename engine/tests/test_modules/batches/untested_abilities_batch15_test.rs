/// Untested-abilities batch 15 — depth=none gaps:
/// - PL!SP-sd1-026-SD / -SRL (ライブ開始時): score +1 while own energy ≥ 9
/// - PL!S-pb1-020-L (ライブ開始時): score +2 when staged Aqours members'
///   combined printed heart04 count is ≥ 10
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

// ====================================================================
// PL!SP-sd1-026-SD / -SRL (ライブ開始時):
// 「自分のエネルギーが9枚以上の場合、このカードのスコアを＋１する。」
// ====================================================================

#[test]
fn sd1026_energy_nine_or_more_scores() {
    let db = load_real_database();
    for variant in ["PL!SP-sd1-026-SD", "PL!SP-sd1-026-SRL"] {
        let mut game = TestGame::new(db.clone());
        let live = game.id(variant);
        game.state.player1.live_card_zone.cards.push(live);
        game.give_energy(9);

        fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

        assert_eq!(
            game.state.mods.get_score_modifier(live),
            1,
            "{}: 9 energy -> score +1",
            variant
        );
    }
}

#[test]
fn sd1026_energy_below_nine_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-sd1-026-SD");
    game.state.player1.live_card_zone.cards.push(live);
    game.give_energy(8);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "8 energy -> no score"
    );
}

// ====================================================================
// PL!S-pb1-020-L 私のSymphony?? no — トレココPLEASE!! (ライブ開始時):
// 「自分のステージにいる『Aqours』のメンバーが持つハートに、
//   {{heart_04}}が合計10つ以上ある場合、このカードのスコアを＋２する。」
// ====================================================================

#[test]
fn treco_please_aqours_h04_total_ten_scores_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-020-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Two Aqours members with 5 printed heart04 each => total 10.
    let a1 = game.id("PL!S-bp5-007-R");
    let a2 = game.new_id("PL!S-bp5-007-R");
    game.state.player1.stage.stage[0] = a1;
    game.state.player1.stage.stage[1] = a2;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "combined printed heart04 = 10 -> score +2"
    );
}

#[test]
fn treco_please_aqours_h04_below_ten_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-020-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Single member with 5 heart04 -> below threshold.
    let a1 = game.id("PL!S-bp5-007-R");
    game.state.player1.stage.stage[0] = a1;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "combined printed heart04 = 5 -> no score"
    );
}
