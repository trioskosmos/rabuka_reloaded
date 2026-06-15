/// Tests for modify_required_hearts_global effect:
/// 必要ハートが多くなる — increase required hearts on all live cards.
///
/// Real card: PL!SP-bp2-010-P (ウィーン・マルガレーテ):
///   常時: 相手のライブカード置き場にあるすべてのライブカードは、
///         成功させるための必要ハートがheart00多くなる。
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

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
