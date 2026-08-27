/// Comprehensive edges for idx68 ability_filter PL!-bp4-002
/// 常時 自分のライブ中のライブカードに、LiveStartもLiveSuccessも持たないカードがあるかぎり、heart06x2を得る。
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn h06(g: &TestGame, id: i16) -> i32 { g.state.mods.get_heart_modifier(id, HeartColor::Heart06) }

#[test]
fn ability_filter_live_zone_empty_no_gain() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let eli = g.id("PL!-bp4-002-R＋");
    g.state.player1.stage.stage[1] = eli;
    // Live zone empty
    g.state.player1.live_card_zone.cards.clear();
    g.state.recalculate_constants();
    assert_eq!(h06(&g, eli), 0, "empty live zone -> no heart");
}

#[test]
fn ability_filter_no_ability_live_gains() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let eli = g.id("PL!-bp4-002-R＋");
    g.state.player1.stage.stage[1] = eli;
    // Live card with is_null true: PL!HS-PR-010-PR has no ability
    let null_live = g.id("PL!HS-PR-010-PR");
    g.state.player1.live_card_zone.cards.push(null_live);
    g.state.recalculate_constants();
    assert_eq!(h06(&g, eli), 2, "one null-ability live -> +2 heart06");
}

#[test]
fn ability_filter_live_with_livestart_no_gain() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let eli = g.id("PL!-bp4-002-R＋");
    g.state.player1.stage.stage[1] = eli;
    // Live card that has LiveStart: PL!HS-PR-018-PR has LiveStart 2 blade
    let livestart_live = g.id("PL!HS-PR-018-PR");
    g.state.player1.live_card_zone.cards.push(livestart_live);
    g.state.recalculate_constants();
    assert_eq!(h06(&g, eli), 0, "live with LiveStart -> no heart");
}

#[test]
fn ability_filter_mixed_one_null_one_livestart_gains() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let eli = g.id("PL!-bp4-002-R＋");
    g.state.player1.stage.stage[1] = eli;
    let null_live = g.id("PL!HS-PR-010-PR");
    let livestart_live = g.id("PL!HS-PR-018-PR");
    g.state.player1.live_card_zone.cards.push(null_live);
    g.state.player1.live_card_zone.cards.push(livestart_live);
    g.state.recalculate_constants();
    assert_eq!(h06(&g, eli), 2, "at least one null-ability live -> +2");
}

#[test]
fn ability_filter_removing_null_loses_heart() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let eli = g.id("PL!-bp4-002-R＋");
    g.state.player1.stage.stage[1] = eli;
    let null_live = g.id("PL!HS-PR-010-PR");
    g.state.player1.live_card_zone.cards.push(null_live);
    g.state.recalculate_constants();
    assert_eq!(h06(&g, eli), 2);
    // Remove null live to success zone (simulate live ends)
    g.state.player1.live_card_zone.cards.clear();
    g.state.player1.success_live_card_zone.cards.push(null_live);
    g.state.recalculate_constants();
    assert_eq!(h06(&g, eli), 0, "null live moved to success -> live zone empty -> no heart");
}

#[test]
fn ability_filter_success_zone_does_not_count() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let eli = g.id("PL!-bp4-002-R＋");
    g.state.player1.stage.stage[1] = eli;
    let null_live = g.id("PL!HS-PR-010-PR");
    // Put null live in success, not live zone
    g.state.player1.success_live_card_zone.cards.push(null_live);
    g.state.recalculate_constants();
    assert_eq!(h06(&g, eli), 0, "success zone null should not count, only live zone");
}
