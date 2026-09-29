//! 南ことり `PL!-pb2-030-N` (常時)
//!
//! 自分の成功ライブカード置き場にあるカードのスコアの合計５につき、{{icon_blade.png|ブレード}}を得る。
//!
//! The unit is **five points of total score**, not one card. So the count has to be
//! the score sum divided by 5, floored: 4 → 0, 5 → 1, 9 → 1, 10 → 2, 12 → 2.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KOTORI: &str = "PL!-pb2-030-N";

/// Stage the member, load the success zone with lives totalling `total_score`,
/// and return the blade modifier. Lives are drawn from a fixed ladder so each total
/// is exact.
fn blade_for_score_total(total_score: i32) -> i16 {
    // Ladder of (score, card) that sums to any target in these tests.
    const LADDER: &[(i32, &str)] = &[
        (9, "PL!S-pb1-023-L"),
        (5, "PL!-pb1-028-L"),
        (4, "PL!-bp6-023-L"),
        (3, "PL!-sd1-021-SD"),
        (2, "PL!-bp4-024-L"),
        (1, "PL!-sd1-019-SD"),
    ];

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    game.assert_card_identity(kotori, KOTORI);
    game.add_to_stage(MemberArea::Center, kotori);

    let mut remaining = total_score;
    for (score, card_no) in LADDER {
        while remaining >= *score {
            game.state
                .player1
                .success_live_card_zone
                .add_card(game.id(card_no));
            remaining -= score;
        }
        if remaining == 0 {
            break;
        }
    }
    assert_eq!(
        remaining, 0,
        "fixture must be able to build a score total of exactly {}",
        total_score
    );

    let mut sum = 0u8;
    for id in &game.state.player1.success_live_card_zone.cards {
        sum += game.db.get_card(*id).and_then(|c| c.score).unwrap_or(0);
    }
    assert_eq!(sum as i32, total_score, "fixture score sum precondition");

    game.state.recalculate_constants();
    game.state.mods.get_blade_modifier(kotori)
}

#[test]
fn score_sum_zero_grants_no_blade() {
    assert_eq!(blade_for_score_total(0), 0, "合計0 is below one 5");
}

#[test]
fn score_sum_four_grants_no_blade() {
    assert_eq!(
        blade_for_score_total(4),
        0,
        "合計4 — just under the 5 boundary, so 0 blades"
    );
}

#[test]
fn score_sum_five_grants_one_blade() {
    assert_eq!(
        blade_for_score_total(5),
        1,
        "合計5につき — the boundary itself is the first blade"
    );
}

#[test]
fn score_sum_nine_still_grants_one_blade() {
    assert_eq!(
        blade_for_score_total(9),
        1,
        "合計9 — one full 5 plus 4 remainder, still only 1 blade (floored, not rounded)"
    );
}

#[test]
fn score_sum_ten_grants_two_blades() {
    assert_eq!(
        blade_for_score_total(10),
        2,
        "合計10 is exactly two 5s, so 2 blades"
    );
}

#[test]
fn score_sum_twelve_grants_two_blades() {
    assert_eq!(
        blade_for_score_total(12),
        2,
        "合計12 is two 5s plus 2 remainder, so 2 blades"
    );
}

#[test]
fn score_sum_fifteen_grants_three_blades() {
    assert_eq!(
        blade_for_score_total(15),
        3,
        "合計15 is three 5s, so 3 blades"
    );
}

/// The unit is the SUM OF SCORES, not the card count. Four low-score lives must not
/// behave like four 5s.
#[test]
fn many_low_score_cards_do_not_behave_like_five_point_units() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    game.add_to_stage(MemberArea::Center, kotori);

    // Four score-1 lives: 4 cards, score sum 4.
    let one = game.id("PL!-sd1-019-SD");
    for i in 0..4 {
        let id = if i == 0 { one } else { game.new_id("PL!-sd1-019-SD") };
        game.state.player1.success_live_card_zone.add_card(id);
    }
    assert_eq!(
        game.state.player1.success_live_card_zone.len(),
        4,
        "precondition: four cards"
    );
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        0,
        "4 cards of score 1 is 合計4, not 4 units of 5 — the count must key on score"
    );
}

/// The zone is 自分の.
#[test]
fn opponent_success_zone_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    game.add_to_stage(MemberArea::Center, kotori);
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id("PL!N-bp4-031-L"));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        0,
        "自分の成功ライブカード置き場 — the opponent's zone must not contribute"
    );
}

#[test]
fn off_stage_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    game.state
        .player1
        .success_live_card_zone
        .add_card(game.id("PL!S-pb1-023-L"));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(kotori),
        0,
        "常時 — off stage she contributes no blade"
    );
}
