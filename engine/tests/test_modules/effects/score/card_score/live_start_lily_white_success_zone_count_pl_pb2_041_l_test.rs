//! 春情ロマンティック `PL!-pb2-041-L` (ab#1) — ライブ開始時
//!
//! 自分の成功ライブカード置き場に『lily white』のカードが2枚以上ある場合、このカードのスコアを＋１する。
//!
//! Covers the `card_count_condition` on `success_live_card_zone` with a group filter:
//! positive, the below-threshold boundary, the not-a-multiplier case, and the group filter itself.

use crate::helpers::*;
use rabuka_engine::game_state::Phase;

/// The subject live. Printed score 7, so any modifier observed is from its ability.
const ROMANTIC: &str = "PL!-pb2-041-L";

/// A `lily white` live cheap enough to sit in the success zone as a prior result.
const LW_CHEAP: &str = "PL!-pb1-029-SRL"; // score 1
/// A second, distinct `lily white` live.
const LW_MID: &str = "PL!-bp5-023-SRL"; // score 3
/// A `lily white` live used for the "three still counts once" case.
const LW_THIRD: &str = "PL!-bp4-025-SRL"; // score 5
/// A non-`lily white` live (μ's) for the group-filter case.
const MUS_LIVE: &str = "PL!-sd1-021-SD"; // score 3

/// Stand the subject live up for a LiveStart, with `success_zone` pre-loaded, and
/// return the live's score modifier as the engine sees it once ライブ開始時 has run.
fn score_mod_with_success_zone(success_zone: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let romantic = game.id(ROMANTIC);
    game.assert_card_identity(romantic, ROMANTIC);
    game.assert_card_score(romantic, 7);

    // Pin the fixture's group membership through the same matcher the ability uses.
    for card_no in success_zone {
        let id = game.id(card_no);
        if *card_no != MUS_LIVE {
            game.assert_card_in_group(id, "lily white", "success-zone fixture");
        }
        game.state.player1.success_live_card_zone.cards.push(id);
    }

    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(romantic);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    fill_decks(&mut game, filler);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(romantic);
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    game.state.mods.get_score_modifier(romantic)
}

/// Two `lily white` cards in the success zone → the printed +1 applies.
#[test]
fn two_lily_white_success_cards_score_plus_one() {
    assert_eq!(
        score_mod_with_success_zone(&[LW_CHEAP, LW_MID]),
        1,
        "自分の成功ライブカード置き場に『lily white』のカードが2枚以上ある場合、\
         このカードのスコアを＋１する — two qualifying cards must give exactly +1"
    );
}

/// One `lily white` card is below the printed 2枚以上 threshold → no bonus.
#[test]
fn one_lily_white_success_card_scores_zero() {
    assert_eq!(
        score_mod_with_success_zone(&[LW_CHEAP]),
        0,
        "the threshold is 2枚以上 — a single qualifying card must not score"
    );
}

/// Three `lily white` cards still give +1. The text is a condition, not a multiplier:
/// 「2枚以上」 must not be read as +1 per card or per pair.
#[test]
fn three_lily_white_success_cards_still_score_one() {
    assert_eq!(
        score_mod_with_success_zone(&[LW_CHEAP, LW_MID, LW_THIRD]),
        1,
        "2枚以上 gates the effect on once — three qualifying cards must still be +1, not +2"
    );
}

/// Two `lily white` cards held in hand rather than the success zone → no bonus.
#[test]
fn lily_white_cards_in_hand_do_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let romantic = game.id(ROMANTIC);
    game.assert_card_identity(romantic, ROMANTIC);

    let lw1 = game.id(LW_CHEAP);
    let lw2 = game.id(LW_MID);
    game.assert_all_in_group(&[lw1, lw2], "lily white", "hand fixture");
    game.add_to_hand(lw1);
    game.add_to_hand(lw2);

    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(romantic);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    fill_decks(&mut game, filler);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(romantic);
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    assert_eq!(
        game.state.mods.get_score_modifier(romantic),
        0,
        "the condition reads 自分の成功ライブカード置き場 only — two lily white cards \
         in hand must not satisfy it"
    );
}

/// Two non-`lily white` success lives → the group filter excludes them → no bonus.
#[test]
fn non_lily_white_success_cards_do_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let romantic = game.id(ROMANTIC);
    game.assert_card_identity(romantic, ROMANTIC);

    let mus1 = game.id(MUS_LIVE);
    let mus2 = game.id(MUS_LIVE);
    game.state.player1.success_live_card_zone.cards.push(mus1);
    game.state.player1.success_live_card_zone.cards.push(mus2);

    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(romantic);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    fill_decks(&mut game, filler);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(romantic);
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    assert_eq!(
        game.state.mods.get_score_modifier(romantic),
        0,
        "『lily white』のカードが2枚以上 — two μ's lives do not satisfy the group filter"
    );
}

/// An empty success zone → no bonus. Covers the "there is a zone but nothing in it"
/// branch, which is distinct from "the zone has the wrong cards".
#[test]
fn empty_success_zone_scores_zero() {
    assert_eq!(
        score_mod_with_success_zone(&[]),
        0,
        "an empty 成功ライブカード置き場 cannot satisfy 2枚以上"
    );
}
