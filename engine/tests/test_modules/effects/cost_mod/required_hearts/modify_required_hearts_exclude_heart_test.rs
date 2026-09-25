/// Tests for PL!-bp5-023-L (乙姫心で恋宮殿) — `modify_required_hearts` with
/// `exclude_heart_colors` + `per_unit`.
///
/// Card ability (ライブ開始時):
///   自分のステージにいる{{heart_01.png|heart01}}と{{heart_06.png|heart06}}以外の色の
///   ハートを持つメンバー1人につき、このカードの必要ハートを{{heart_00.png|heart0}}減らす。
///
/// Translation:
///   On LiveStart: for each member on your stage that POSSESSES a heart color
///   OTHER THAN heart01 and heart06, reduce this card's required heart00 by 1.
///
/// Key grammar detail: "heart01とheart06以外の色のハートを持つ" means the member
/// must have AT LEAST ONE heart color that is NOT heart01 and NOT heart06.
/// A member with {heart01, heart06, heart03} STILL counts because heart03
/// is a color outside {heart01, heart06}.
///
/// Base requirement: heart01=3, heart06=2, heart0=3
/// Per eligible member: heart00 -= 1
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

/// Advance from the initial phase through to LiveCardSetP1.
fn advance_to_live_card_set(game: &mut TestGame) {
    game.pass(); // → ActivePhase
    game.pass(); // → EnergyPhase
    game.pass(); // → DrawPhase
    game.pass(); // → MainPhase
    game.pass(); // → LiveCardSetP1
}

/// Advance from LiveCardSetP1 through LiveStart (triggers fire here).
fn finish_live_setup(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → LiveStart (triggers fire here)
    game.drain_auto_ability_choices();
}

// ─────────────────────────────────────────────────────────────
// Test 1: All stage members have ONLY excluded heart colors
// → 0 eligible members → NO reduction.
// Members:
//   PL!-sd1-002-SD (only heart06) → excluded
//   PL!-PR-002-PR  (only heart06) → excluded
//   PL!-sd1-005-SD (only heart01) → excluded
// ─────────────────────────────────────────────────────────────
#[test]
fn all_members_excluded_no_reduction() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-023-L");
    let excluded1 = game.id("PL!-sd1-002-SD"); // heart06 only
    let excluded2 = game.id("PL!-PR-002-PR"); // heart06 only
    let excluded3 = game.id("PL!-sd1-005-SD"); // heart01 only
    let filler = game.id("LL-E-001-SD"); // energy card (no base_heart)

    game.state.player1.stage.stage = [excluded1, excluded2, excluded3];
    game.state.player1.hand.cards.push(card);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(card);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(card, HeartColor::Heart00);
    assert_eq!(
        mod_val, 0,
        "All members excluded → heart00 should have 0 reduction, got {mod_val}"
    );
}

// ─────────────────────────────────────────────────────────────
// Test 2: One non-excluded member → heart00 -= 1
// Members:
//   PL!-sd1-001-SD (heart01+03+06) → has heart03 → counts
//   PL!-sd1-002-SD (only heart06)  → excluded
//   PL!-PR-002-PR  (only heart06)  → excluded
// ─────────────────────────────────────────────────────────────
#[test]
fn one_non_excluded_member_reduces_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-023-L");
    let counts = game.id("PL!-sd1-001-SD"); // heart01+03+06 → has heart03 → counts
    let excl1 = game.id("PL!-sd1-002-SD"); // heart06 only → excluded
    let excl2 = game.id("PL!-PR-002-PR"); // heart06 only → excluded
    let filler = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [counts, excl1, excl2];
    game.state.player1.hand.cards.push(card);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(card);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(card, HeartColor::Heart00);
    assert_eq!(
        mod_val, -1,
        "1 eligible member → heart00 should be -1, got {mod_val}"
    );
}

// ─────────────────────────────────────────────────────────────
// Test 3: Mixed — excluded + non-excluded members → heart00 -= 2
// Members:
//   PL!-sd1-002-SD (only heart06)  → excluded
//   PL!-sd1-001-SD (heart01+03+06) → counts (has heart03)
//   PL!-sd1-010-SD (heart01+03)    → counts (has heart03)
// ─────────────────────────────────────────────────────────────
#[test]
fn mixed_excluded_and_counted_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-023-L");
    let excl = game.id("PL!-sd1-002-SD"); // heart06 only → excluded
    let cnt1 = game.id("PL!-sd1-001-SD"); // heart01+03+06 → counts
    let cnt2 = game.id("PL!-sd1-010-SD"); // heart01+03 → counts
    let filler = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [excl, cnt1, cnt2];
    game.state.player1.hand.cards.push(card);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(card);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(card, HeartColor::Heart00);
    assert_eq!(
        mod_val, -2,
        "2 eligible members → heart00 should be -2, got {mod_val}"
    );
}

// ─────────────────────────────────────────────────────────────
// Test 4: Edge case — member with heart01 AND heart06 AND heart03
// STILL counts because it has a color outside {heart01, heart06}.
// → heart00 -= 1
// Members:
//   PL!-sd1-001-SD (heart01+03+06) → counts (has heart03)
//   PL!-sd1-002-SD (only heart06)  → excluded
//   PL!-PR-002-PR  (only heart06)  → excluded
// ─────────────────────────────────────────────────────────────
#[test]
fn member_with_both_excluded_and_non_excluded_colors_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-023-L");
    let member = game.id("PL!-sd1-001-SD"); // heart01+03+06 → has heart03 → counts
    let excl1 = game.id("PL!-sd1-002-SD"); // heart06 only → excluded
    let excl2 = game.id("PL!-PR-002-PR"); // heart06 only → excluded
    let filler = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [member, excl1, excl2];
    game.state.player1.hand.cards.push(card);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(10);

    advance_to_live_card_set(&mut game);
    game.set_live_card(card);
    finish_live_setup(&mut game);

    let mod_val = game
        .state
        .mods
        .get_need_heart_modifier(card, HeartColor::Heart00);
    assert_eq!(
        mod_val, -1,
        "Member with heart01+03+06 counts (has heart03) → heart00 should be -1, got {mod_val}"
    );
}
