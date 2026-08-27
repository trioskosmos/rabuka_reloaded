// Tests for the b_heart07 blade-heart mechanic (colorless hearts).
//
// Mechanic: `blade_heart: { "b_heart07": 1 }` contributes 2 COLORLESS (heart0)
// hearts. A colorless heart can ONLY be used to replace heart0 requirements of
// a live card — it can NEVER be used as a specific color (heart01-heart06).
// See rule 2.1.1.2 / 2.11.3 in engine/rules/rules.txt.
use crate::helpers::*;
use rabuka_engine::card::{parse_heart_color, HeartColor};

// A b_heart07 card used as a yell reveal: PL!N-bp7-030-L (Cheer Mode).
// blade_heart = { b_heart07: 1 } → contributes 2 colorless hearts when revealed.
const B_HEART07_CARD: &str = "PL!N-bp7-030-L";

// Live cards used in the tests (simple need_heart, score > 0, no interfering
// auto-abilities):
//   PL!-sd1-020-SD  きっと青春が聞こえる  need {heart01:1, heart03:1, heart0:3}
//   PL!HS-bp1-019-L Dream Believers       need {heart0:4}
const COLORED_LIVE: &str = "PL!-sd1-020-SD";
const HEART0_ONLY_LIVE: &str = "PL!HS-bp1-019-L";

// Stage members (blade=2 so 2 yell cards are revealed):
//   PL!SP-pb1-014-PR 嵐 千砂都   blade=2, base {heart06:1} — NO heart01/heart03
//   PL!-bp4-012-N    南 ことり    blade=2, base {heart01:1, heart03:2} — has colors
const MEMBER_NO_COLOR: &str = "PL!SP-pb1-014-PR";
const MEMBER_WITH_COLOR: &str = "PL!-bp4-012-N";

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "expected LiveCardSet phase, got {:?}",
        game.state.current_phase
    );
}

fn advance_to_live_success(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Fill a deck so that:
///   - index 0 is drawn away during the Draw phase,
///   - indices 1..=blade are the yell reveals (all B_HEART07_CARD),
///   - the rest are filler (never revealed because blade limits the yell).
fn setup_deck_with_b_heart07_yells(game: &mut TestGame, blade: usize) {
    game.state
        .player1
        .main_deck
        .cards
        .push(game.id("PL!-sd1-010-SD")); // index 0 → hand
    for _ in 0..blade {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id(B_HEART07_CARD));
    }
    for _ in 0..(10 - 1 - blade) {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    for _ in 0..10 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
}

/// `b_heart07` parses to the COLORLESS heart (Heart00), exactly like heart0.
#[test]
fn b_heart07_parses_to_colorless_heart00() {
    assert_eq!(parse_heart_color("b_heart07"), HeartColor::Heart00);
    assert_eq!(
        "b_heart07".parse::<HeartColor>().unwrap(),
        HeartColor::Heart00
    );
    assert_eq!(HeartColor::Heart00.index(), 0, "colorless = pool index 0");
}

/// A revealed b_heart07 card contributes 2 colorless hearts (×2), NOT 1.
/// Two reveals → total_hearts[0] == 4 and each yell card reports blade_hearts[0] == 2.
#[test]
fn b_heart07_yell_contributes_two_colorless_hearts_each() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let live = game.id(HEART0_ONLY_LIVE); // need {heart0: 4}
    game.add_to_hand(live);
    game.add_to_stage(
        rabuka_engine::zones::MemberArea::Center,
        game.id(MEMBER_NO_COLOR), // blade=2
    );
    setup_deck_with_b_heart07_yells(&mut game, 2);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_success(&mut game);

    let snap = &game.state.performance_snapshots[0];
    assert_eq!(snap.yell_cards.len(), 2, "two yell reveals");
    for yc in &snap.yell_cards {
        assert_eq!(yc.blade_hearts[0], 2, "each b_heart07 → 2 colorless hearts");
    }
    assert_eq!(
        snap.total_hearts[0], 4,
        "two b_heart07 cards → 4 colorless hearts in the pool"
    );
    assert!(snap.lives[0].passed, "4 colorless hearts satisfy heart0: 4");
    assert!(snap.success);
}

/// Colorless hearts alone CANNOT satisfy a colored (heart01/heart03) note.
/// Total hearts are sufficient (5), but the specific colors are missing → FAIL.
#[test]
fn colorless_hearts_cannot_fill_colored_requirement() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // Member has heart06 only; yell provides 4 colorless. Pool total = 5 = required,
    // but heart01 and heart03 are absent and colorless hearts cannot become them.
    let live = game.id(COLORED_LIVE);
    game.add_to_hand(live);
    game.add_to_stage(
        rabuka_engine::zones::MemberArea::Center,
        game.id(MEMBER_NO_COLOR), // heart06:1, blade=2
    );
    setup_deck_with_b_heart07_yells(&mut game, 2);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_success(&mut game);

    let snap = &game.state.performance_snapshots[0];
    assert_eq!(
        snap.total_hearts.iter().sum::<u8>(),
        5,
        "pool has enough TOTAL hearts"
    );
    assert_eq!(snap.total_hearts[0], 4, "4 colorless hearts are present");
    assert!(
        !snap.lives[0].passed,
        "colorless hearts must NOT fill heart01/heart03 notes"
    );
    assert!(!snap.success);
}

/// When the colored hearts ARE present, colorless hearts fill the heart0 bucket
/// and the live succeeds.
#[test]
fn colorless_hearts_fill_heart0_when_colored_hearts_present() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // Member has heart01:1 + heart03:2 (covers the colored notes); yell provides
    // 4 colorless hearts that cover the heart0: 3 bucket.
    let live = game.id(COLORED_LIVE);
    game.add_to_hand(live);
    game.add_to_stage(
        rabuka_engine::zones::MemberArea::Center,
        game.id(MEMBER_WITH_COLOR), // heart01:1, heart03:2, blade=2
    );
    setup_deck_with_b_heart07_yells(&mut game, 2);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_success(&mut game);

    let snap = &game.state.performance_snapshots[0];
    assert!(
        snap.lives[0].passed,
        "colored notes filled + heart0 filled by colorless"
    );
    assert!(snap.success);
}

/// Insufficient colorless hearts fail a heart0-only live: 1 b_heart07 reveal gives
/// only 2 colorless hearts, but Dream Believers needs heart0: 4.
#[test]
fn insufficient_colorless_hearts_fail_heart0_only_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    // blade=1 member (PL!-sd1-002-SD: heart06:1) → 1 yell reveal → 2 colorless.
    let live = game.id(HEART0_ONLY_LIVE); // need {heart0: 4}
    game.add_to_hand(live);
    game.add_to_stage(
        rabuka_engine::zones::MemberArea::Center,
        game.id("PL!-sd1-002-SD"), // blade=1
    );
    setup_deck_with_b_heart07_yells(&mut game, 1);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_success(&mut game);

    let snap = &game.state.performance_snapshots[0];
    assert_eq!(
        snap.total_hearts[0], 2,
        "only 2 colorless hearts from one reveal"
    );
    assert!(
        !snap.lives[0].passed,
        "2 colorless hearts are not enough for heart0: 4"
    );
    assert!(!snap.success);
}
