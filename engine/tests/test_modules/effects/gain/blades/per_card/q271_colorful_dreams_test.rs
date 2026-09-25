/// Q271 — Colorful Dreams! Colorful Smiles! PL!N-bp7-025-L.
///
/// ab#0 (ライブ開始時): ライブ終了時まで、自分のステージにいる『虹ヶ咲』のメンバー1人は、
///   ブレードを得る。
/// ab#1 (ライブ成功時): エールにより公開された自分のカードの中に heart01〜heart06 のうち
///   3種類以上ある場合、このカードのスコアを＋１する。
///
/// Official QA Q271: エールにより公開されたカードが【桃ブレードハート】【青ブレードハート】
/// 【ALLブレードハート】を持っていました。このとき、[ab#1]の条件を満たしますか？
/// → いいえ。満たしません。 Blade-hearts (including ALL) are NOT heart colors; only
/// base-heart colors count toward the "3+ distinct types" condition.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const CD: &str = "PL!N-bp7-025-L";
// Single base-heart member cards (no blade).
const H01: &str = "PL!-sd1-005-SD";
const H03: &str = "PL!-PR-001-PR";
const H06: &str = "PL!-sd1-002-SD";
// Multi base-heart member card (heart01+heart03+heart06) — one card = 3 types.
const H136: &str = "PL!-sd1-003-SD";
// Blade-only live cards (NO base heart) — Q271's exact example colors.
const BLADE_PEACH: &str = "PL!-bp3-022-L"; // b_heart01 (peach)
const BLADE_BLUE: &str = "PL!N-bp1-027-L"; // b_heart05 (blue)
const BLADE_ALL: &str = "PL!-sd1-020-SD"; // b_all
const BLADE_YELLOW: &str = "PL!-sd1-022-SD"; // b_heart03 (yellow)
// 虹ヶ咲 member (ab#0 target) and a non-虹ヶ咲 member (should NOT gain blade).
const NIJI_MEMBER: &str = "PL!N-sd1-004-SD";
const NON_NIJI_MEMBER: &str = "PL!-sd1-010-SD";

fn score(g: &TestGame, id: i16) -> i32 {
    g.state.mods.get_score_modifier(id)
}

/// Put CD in the live zone, reveal the given cards (deck card_nos), trigger
/// ab#1 (ライブ成功時), return CD's id.
fn trigger_success(game: &mut TestGame, revealed: &[&str]) -> i16 {
    let cd = game.id(CD);
    game.state.player1.live_card_zone.cards.push(cd);
    for &r in revealed {
        game.state.revealed_cards.push(game.id(r));
    }
    let card = game.db.get_card(cd).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .expect("ab#1 should be ライブ成功時");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cd),
        None,
        None,
    );
    game.state.activating_card = Some(cd);
    game.state.process_pending_auto_abilities(&pid);
    cd
}

/// Trigger ab#0 (ライブ開始時) with the given member on p1 center. Returns (cd, member).
fn trigger_start(game: &mut TestGame, member: &str) -> (i16, i16) {
    let cd = game.id(CD);
    let m = game.id(member);
    game.state.player1.live_card_zone.cards.push(cd);
    game.state.player1.stage.stage[1] = m;
    let card = game.db.get_card(cd).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
        .expect("ab#0 should be ライブ開始時");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cd),
        None,
        None,
    );
    game.state.activating_card = Some(cd);
    game.state.process_pending_auto_abilities(&pid);
    (cd, m)
}

// ====================================================================
// ab#1 — Q271: blade-hearts are NOT heart colors.
// ====================================================================

/// Q271 core: revealed cards have ONLY 桃/青/ALL blade-hearts (no base hearts)
/// → does NOT satisfy the "3+ types of heart color" condition → no score.
#[test]
fn q271_blade_hearts_do_not_satisfy_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Peach + blue + ALL blade-hearts: would be "3 colors" if blade counted.
    let cd = trigger_success(&mut game, &[BLADE_PEACH, BLADE_BLUE, BLADE_ALL]);

    assert_eq!(
        score(&game, cd),
        0,
        "Q271: 桃/青/ALL blade-hearts are not heart colors → condition NOT met"
    );
}

/// Three distinct blade-hearts that each map to a heart color (peach/yellow/blue)
/// → still NOT satisfied (blade-hearts never count as heart colors).
#[test]
fn q271_blade_hearts_mapping_to_colors_still_not_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // b_heart01→peach, b_heart03→yellow, b_heart05→blue — 3 blade colors.
    let cd = trigger_success(&mut game, &[BLADE_PEACH, BLADE_YELLOW, BLADE_BLUE]);

    assert_eq!(
        score(&game, cd),
        0,
        "Q271: even 3 blade-heart colors do not count as heart colors"
    );
}

/// 3 distinct BASE heart colors among the revealed cards → satisfied → +1 score.
#[test]
fn q271_three_distinct_base_hearts_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let cd = trigger_success(&mut game, &[H01, H03, H06]);

    assert_eq!(
        score(&game, cd),
        1,
        "3 distinct base heart colors (heart01/03/06) → +1 score"
    );
}

/// A single card with 3 base heart colors also counts as 3 types.
#[test]
fn q271_one_card_with_three_base_hearts_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let cd = trigger_success(&mut game, &[H136]);

    assert_eq!(
        score(&game, cd),
        1,
        "one revealed card with heart01+heart03+heart06 → 3 types → +1 score"
    );
}

/// Only 2 distinct base heart colors → NOT satisfied → no score.
#[test]
fn q271_two_base_hearts_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let cd = trigger_success(&mut game, &[H01, H03]);

    assert_eq!(
        score(&game, cd),
        0,
        "2 distinct base heart colors → condition (>=3) NOT met → no score"
    );
}

/// Blade-hearts don't add a color: 2 base hearts + blade-only cards is still 2.
#[test]
fn q271_blade_hearts_do_not_add_a_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // heart01, heart03 (2 base) + blade-only peach & ALL → still 2 types.
    let cd = trigger_success(&mut game, &[H01, H03, BLADE_PEACH, BLADE_ALL]);

    assert_eq!(
        score(&game, cd),
        0,
        "blade-hearts don't add a color: 2 base + blade-only is still 2 types → no score"
    );
}

/// No revealed cards → condition not met (0 types) → no score.
#[test]
fn q271_no_revealed_cards_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let cd = trigger_success(&mut game, &[]);

    assert_eq!(score(&game, cd), 0, "no revealed cards → no score");
}

// ====================================================================
// ab#0 — ライブ開始時: a 虹ヶ咲 member gains 1 blade until live end.
// ====================================================================

/// A 虹ヶ咲 member on stage gains +1 blade.
#[test]
fn q271_ab0_niji_member_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (_cd, m) = trigger_start(&mut game, NIJI_MEMBER);

    assert_eq!(
        game.state.mods.get_blade_modifier(m),
        1,
        "ab#0: 虹ヶ咲 member gains 1 blade"
    );
}

/// A non-虹ヶ咲 member does NOT gain blade (group filter).
#[test]
fn q271_ab0_non_niji_member_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let (_cd, m) = trigger_start(&mut game, NON_NIJI_MEMBER);

    assert_eq!(
        game.state.mods.get_blade_modifier(m),
        0,
        "ab#0: non-虹ヶ咲 member must NOT gain blade"
    );
}
