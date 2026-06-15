/// Tests for modify_required_hearts_global effect:
/// 必要ハートが多くなる — increase required hearts on all live cards.
///
/// Real card: PL!SP-bp2-010-P (ウィーン・マルガレーテ):
///   常時: 相手のライブカード置き場にあるすべてのライブカードは、
///         成功させるための必要ハートがheart00多くなる。
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

/// Wien on stage, opponent has 1 live card → heart00 requirement increases by 1.
#[test]
fn wien_constant_increases_opponent_live_cards_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp2-010-P");
    let opponent_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, wien, -1];
    game.state.player2.live_card_zone.cards.push(opponent_live);

    game.state.recalculate_constants();

    let heart00_mod = game
        .state
        .mods
        .get_need_heart_modifier(opponent_live, HeartColor::Heart00);
    assert!(
        heart00_mod >= 1,
        "Wien constant: opponent live card should have +1 heart00, got {}",
        heart00_mod
    );
}

/// When Wien leaves stage, the modifier should be removed.
#[test]
fn wien_constant_removed_when_leaves_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp2-010-P");
    let opponent_live = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage = [-1, wien, -1];
    game.state.player2.live_card_zone.cards.push(opponent_live);
    game.state.recalculate_constants();

    let mod_before = game
        .state
        .mods
        .get_need_heart_modifier(opponent_live, HeartColor::Heart00);
    assert!(
        mod_before >= 1,
        "Modifier should be active while Wien is on stage"
    );

    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.recalculate_constants();

    let mod_after = game
        .state
        .mods
        .get_need_heart_modifier(opponent_live, HeartColor::Heart00);
    assert_eq!(
        mod_after, 0,
        "Modifier removed after Wien leaves, got {}",
        mod_after
    );
}

/// Multiple opponent live cards all get the +1 heart00 modifier.
#[test]
fn wien_constant_applies_to_all_opponent_live_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let wien = game.id("PL!SP-bp2-010-P");
    let live1 = game.id("PL!-sd1-019-SD");
    let live2 = game.id("PL!-sd1-020-SD");
    let live3 = game.id("PL!-sd1-021-SD");

    game.state.player1.stage.stage = [-1, wien, -1];
    game.state.player2.live_card_zone.cards = vec![live1, live2, live3].into();

    game.state.recalculate_constants();

    for (i, &card) in [live1, live2, live3].iter().enumerate() {
        let mod_val = game
            .state
            .mods
            .get_need_heart_modifier(card, HeartColor::Heart00);
        assert!(
            mod_val >= 1,
            "Opponent live card {} should have +1 heart00, got {}",
            i,
            mod_val
        );
    }
}

// ====================================================================
// PL!S-bp5-011-N (桜内梨子) — 登場: conditional global heart increase
// ====================================================================
// 登場: 自分のステージにいるメンバーが持つハートにheart05が合計5つ以上ある場合、
// 相手のライブ開始時、相手のライブカード置き場にあるライブカード1枚は、
// 成功させるための必要ハートがheart0多くなる。
// ====================================================================

/// Condition met (total heart05 >= 5): opponent's live card gets +1 heart00.
#[test]
fn riko_bp5_condition_met_increases_opponent_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-sd1-001-SD");   // heart05=2
    let riko_sd = game.id("PL!S-sd1-011-SD");  // heart05=2
    let riko_bp5 = game.id("PL!S-bp5-011-N");  // heart05=1, 登場 trigger
    let opp_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // 2 stage members + 1 more from play_to_stage = 3 with total heart05 = 2+2+1 = 5
    game.state.player1.stage.stage[0] = chika;
    game.state.player1.stage.stage[1] = riko_sd;
    game.state.player2.live_card_zone.cards.push(opp_live);

    game.add_to_hand(riko_bp5);
    game.add_to_hand(filler);
    game.give_energy(10);

    game.play_to_stage(riko_bp5, MemberArea::RightSide);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(opp_live, HeartColor::Heart00);
    assert_eq!(
        mod_val, 1,
        "Condition met (heart05=5): Riko should increase opponent's heart00 by 1"
    );
}

/// Condition NOT met (heart05 < 5): no modifier applied.
#[test]
fn riko_bp5_condition_not_met_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let riko_sd = game.id("PL!S-sd1-011-SD");  // heart05=2
    let riko_bp5 = game.id("PL!S-bp5-011-N");  // heart05=1 (only this + riko_sd after play)
    let opp_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Only 1 member on stage with heart05=2; after playing riko_bp5 total = 2+1 = 3 < 5
    game.state.player1.stage.stage[0] = riko_sd;
    game.state.player2.live_card_zone.cards.push(opp_live);

    game.add_to_hand(riko_bp5);
    game.add_to_hand(filler);
    game.give_energy(10);

    game.play_to_stage(riko_bp5, MemberArea::RightSide);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(opp_live, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "Condition not met (heart05=3): no modifier should be applied"
    );
}
