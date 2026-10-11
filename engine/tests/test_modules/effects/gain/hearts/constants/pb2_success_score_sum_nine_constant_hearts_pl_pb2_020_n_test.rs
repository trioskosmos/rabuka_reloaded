//! 絢瀬絵里 `PL!-pb2-020-N` (常時)
//!
//! 自分の成功ライブカード置き場にあるカードのスコアの合計が９以上であるかぎり、
//! {{heart_01.png|heart01}}{{heart_03.png|heart03}}を得る。
//!
//! The aggregate is the SUM OF SCORES, not the card count, and the threshold is 9.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-020-N";
/// Score 9 — exactly the printed threshold.
const LIVE9: &str = "PL!S-pb1-023-L";
/// Score 3.
const LIVE3: &str = "PL!-sd1-021-SD";
/// Score 0 — six of these are 6 cards but a score sum of 0.
const LIVE0: &str = "PL!-bp3-019-L";

#[test]
fn score_sum_nine_grants_both_printed_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    game.add_to_stage(MemberArea::Center, eri);

    let live9 = game.id(LIVE9);
    game.assert_card_score(live9, 9);
    game.state.player1.success_live_card_zone.add_card(live9);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart01),
        1,
        "スコアの合計が9以上 — heart01 is granted once"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart03),
        1,
        "スコアの合計が9以上 — heart03 is granted once"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart06),
        0,
        "no collateral heart06 — the card prints heart01 and heart03 only"
    );
}

#[test]
fn score_sum_eight_does_not_cross_the_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);

    // 3 + 3 + 3 = 9 crosses; 3 + 3 = 6 does not.
    let live3a = game.id(LIVE3);
    let live3b = game.new_id(LIVE3);
    game.state.player1.success_live_card_zone.add_card(live3a);
    game.state.player1.success_live_card_zone.add_card(live3b);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.player1.success_live_card_zone.len(),
        2,
        "precondition: two score-3 lives sit in the zone"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart01),
        0,
        "score sum 6 < 9 — かぎり is not yet satisfied"
    );

    let live3c = game.new_id(LIVE3);
    game.state.player1.success_live_card_zone.add_card(live3c);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart01),
        1,
        "score sum 9 reaches the threshold and heart01 appears"
    );
}

/// The gate is the score SUM. Many score-0 lives must not satisfy 9.
#[test]
fn many_zero_score_lives_do_not_reach_nine() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);

    let live0 = game.id(LIVE0);
    for i in 0..9 {
        let id = if i == 0 { live0 } else { game.new_id(LIVE0) };
        game.state.player1.success_live_card_zone.add_card(id);
    }
    assert_eq!(game.state.player1.success_live_card_zone.len(), 9);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart01),
        0,
        "9 cards whose scores total 0 is not スコア9以上 — the gate is the sum, not the count"
    );
}

/// かぎり: the bonus appears and disappears with the condition.
#[test]
fn hearts_are_removed_when_the_sum_drops() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);

    game.state.player1.success_live_card_zone.add_card(game.id(LIVE9));
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart03),
        1,
        "precondition: sum 9 grants heart03"
    );

    game.state.player1.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart03),
        0,
        "かぎり — an empty zone drops the sum to 0 and the constant must lapse"
    );
}

/// The zone is 自分の — the opponent's high-scoring lives must not count.
#[test]
fn opponent_success_zone_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.add_to_stage(MemberArea::Center, eri);
    game.state
        .player2
        .success_live_card_zone
        .add_card(game.id(LIVE9));
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart01),
        0,
        "自分の成功ライブカード置き場 — the opponent's zone is not 自分"
    );
}

/// 常時 only applies while she is on stage.
#[test]
fn off_stage_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    game.state.player1.success_live_card_zone.add_card(game.id(LIVE9));
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_heart_modifier(eri, HeartColor::Heart01),
        0,
        "常時 — an off-stage member contributes nothing"
    );
}

#[test]
fn two_copies_each_grant_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let a = game.id(ERI);
    let b = game.new_id(ERI);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[2] = b;
    game.state.player1.success_live_card_zone.add_card(game.id(LIVE9));
    game.state.recalculate_constants();

    for &copy in &[a, b] {
        assert_eq!(
            game.state.mods.get_heart_modifier(copy, HeartColor::Heart01),
            1,
            "each on-stage copy independently satisfies the same condition"
        );
    }
}
