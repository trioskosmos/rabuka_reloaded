/// Coverage for §5.5 — PL!SP-pb2-048-L ディストーション (idx 867)
/// {{ライブ開始時}}自分のステージにいる名前の異なる『CatChu!』のメンバー1人につき、
/// このカードの必要ハートを{{heart0}}×2減らし、{{heart02}}増やす。
/// その後、このカードの必要ハートに含まれる{{heart02}}が9以上の場合、このカードのスコアを＋１する。
///
/// Parsed (abilities.json idx 867): sequential [
///   modify_required_hearts(heart00, decrease, value=2, per_unit distinct CatChu),
///   modify_required_hearts(heart02, decrease?, value=1, per_unit) — NOTE: parser
///     emitted "decrease" for the 増やす (increase) half — suspected parser bug,
///   modify_score(+1, per_unit) — gated on heart02>=9 in need_heart
/// ]
///
/// Live base: need_heart {heart02:6, heart0:9}, score 6.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn fill_decks(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

fn need_heart(game: &TestGame, live: i16, color: HeartColor) -> u8 {
    // Effective need_heart = base from DB + modifier from mods
    let base = game
        .state
        .card_database
        .get_card(live)
        .and_then(|c| c.need_heart.as_ref())
        .and_then(|nh| nh.hearts.get(&color))
        .copied()
        .unwrap_or(0);
    let modifier = game.state.mods.get_need_heart_modifier(live, color);
    (base as i32 + modifier).max(0) as u8
}

fn live_score_mod(game: &TestGame, live: i16) -> i32 {
    game.state.mods.get_score_modifier(live)
}

fn setup_with_members(members: &[&str]) -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-pb2-048-L");
    fill_decks(&mut game);
    // live card in live zone; members on stage
    game.state.player1.live_card_zone.cards.push(live);
    for (i, m) in members.iter().enumerate() {
        game.state.player1.stage.stage[i] = game.id(m);
    }
    (game, live)
}

/// Base parse audit: the compound per-unit half must be a nested sequential
/// [heart00 decrease ×2, heart02 increase ×1], and the score must be gated on
/// need_heart heart02>=9 (location=live_card_zone, aggregate=total).
#[test]
fn distortion_parse_increase_half_is_increase() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let live = g.id("PL!SP-pb2-048-L");
    let card = g.db.get_card(live).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("distortion should have ライブ開始時");
    let eff = ab.effect.as_ref().expect("has effect");
    let actions = eff.compound.actions.as_ref().expect("sequential actions");
    assert_eq!(actions.len(), 2, "2 top-level actions (per-unit pair + gated score)");

    // Action 0: nested sequential with the two need-heart modifications
    let first = &actions[0];
    assert_eq!(
        first.action,
        rabuka_engine::ability::enums::ActionType::Sequential,
        "action0 must be the nested per-unit sequential"
    );
    let subs = first.compound.actions.as_ref().expect("nested actions");
    assert_eq!(subs.len(), 2);
    assert_eq!(subs[0].operation_any(), Some("decrease"), "step0 heart00 decrease");
    assert_eq!(subs[0].value_or_count(0), 2, "step0 value=2 (heart00 x2)");
    assert_eq!(
        subs[0].heart_colors_any(),
        ["heart00".to_string()].as_slice(),
        "step0 color=heart00"
    );
    assert_eq!(subs[1].operation_any(), Some("increase"), "step1 heart02 増やす=increase");
    assert_eq!(subs[1].value_or_count(0), 1, "step1 value=1");
    assert_eq!(
        subs[1].heart_colors_any(),
        ["heart02".to_string()].as_slice(),
        "step1 color=heart02"
    );

    // Action 1: modify_score gated on heart02>=9 in live_card_zone
    let second = &actions[1];
    assert_eq!(second.action, rabuka_engine::ability::enums::ActionType::ModifyScore);
    assert_eq!(second.value_or_count(0), 1);
    let cond = second.condition.as_ref().expect("score gate condition");
    assert_eq!(cond.get_location(), Some("live_card_zone"), "gate reads live card need_heart");
    assert_eq!(cond.get_aggregate(), Some("total"), "gate sums hearts, not cards");
    assert_eq!(cond.get_count(), Some(9), "gate threshold heart02>=9");
}

/// 3 distinct CatChu names on stage → per-unit=3:
///   heart00: 9 - 2*3 = 3, heart02: 6 + 1*3 = 9 → gate met → score +1
#[test]
fn distortion_three_distinct_catchu_reduces_and_scores() {
    let (mut game, live) = setup_with_members(&[
        "PL!SP-bp1-012-N", // 澁谷かのん
        "PL!SP-bp1-015-N", // 平安名すみれ
        "PL!SP-bp1-018-N", // 米女メイ
    ]);
    let before_h00 = need_heart(&game, live, HeartColor::Heart00);
    let before_h02 = need_heart(&game, live, HeartColor::Heart02);
    assert_eq!(before_h00, 9, "base heart00=9");
    assert_eq!(before_h02, 6, "base heart02=6");

    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let after_h00 = need_heart(&game, live, HeartColor::Heart00);
    let after_h02 = need_heart(&game, live, HeartColor::Heart02);
    assert_eq!(after_h00, 3, "heart00 9 - 2*3 = 3, got {}", after_h00);
    assert_eq!(after_h02, 9, "heart02 6 + 1*3 = 9, got {}", after_h02);
    assert_eq!(
        live_score_mod(&game, live),
        1,
        "heart02>=9 gate met → score+1"
    );
}

/// 1 distinct CatChu name → per-unit=1:
///   heart00: 9-2=7, heart02: 6+1=7 → gate NOT met (7<9) → no score
#[test]
fn distortion_one_catchu_no_score_gate_not_met() {
    let (mut game, live) = setup_with_members(&["PL!SP-bp1-012-N"]);
    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    let after_h00 = need_heart(&game, live, HeartColor::Heart00);
    let after_h02 = need_heart(&game, live, HeartColor::Heart02);
    assert_eq!(after_h00, 7, "heart00 9-2*1=7");
    assert_eq!(after_h02, 7, "heart02 6+1*1=7");
    assert_eq!(
        live_score_mod(&game, live),
        0,
        "heart02=7 < 9 → no score"
    );
}

/// Duplicate names dedupe: kanon ×2 counts as 1 unit.
#[test]
fn distortion_duplicate_names_dedupe() {
    let (mut game, live) = setup_with_members(&[
        "PL!SP-bp1-012-N", // kanon
        "PL!SP-bp4-012-N", // kanon again (different card, same name)
    ]);
    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    let after_h00 = need_heart(&game, live, HeartColor::Heart00);
    let after_h02 = need_heart(&game, live, HeartColor::Heart02);
    assert_eq!(after_h00, 7, "duplicate names → 1 unit: 9-2=7");
    assert_eq!(after_h02, 7, "duplicate names → 1 unit: 6+1=7");
    assert_eq!(live_score_mod(&game, live), 0, "gate not met");
}

/// No CatChu members → no modification at all.
#[test]
fn distortion_no_catchu_no_change() {
    let (mut game, live) = setup_with_members(&["PL!-sd1-010-SD"]);
    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(need_heart(&game, live, HeartColor::Heart00), 9, "unchanged");
    assert_eq!(need_heart(&game, live, HeartColor::Heart02), 6, "unchanged");
    assert_eq!(live_score_mod(&game, live), 0);
}
