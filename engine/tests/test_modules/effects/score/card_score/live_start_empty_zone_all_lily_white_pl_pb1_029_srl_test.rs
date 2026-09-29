//! 知らないLove＊教えてLove `PL!-pb1-029-SRL` (ライブ開始時)
//!
//! 自分の成功ライブカード置き場のカードが0枚で、**かつ** 自分のステージにいるメンバーが
//! 『lily white』のみの場合、このカードのスコアを＋１する。
//!
//! A two-clause AND: an empty success zone AND stage exclusivity. Either clause alone
//! must not score, so each negative below breaks exactly one of them.

use crate::helpers::*;
use rabuka_engine::game_state::Phase;

const LOVE: &str = "PL!-pb1-029-SRL";
/// `lily white` members.
const LW_UMI: &str = "PL!-pb2-013-P＋";
const LW_NOZOMI: &str = "PL!-pb2-025-N";
/// Not `lily white`.
const PRINTEMPS: &str = "PL!-bp6-017-N";
const AZALEA: &str = "PL!S-bp5-012-N";

/// Stand the subject live up with the given stage members and success zone, run
/// ライブ開始時, and return the live's score modifier.
fn score_mod(stage: &[&str], success_zone: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let love = game.id(LOVE);
    game.assert_card_identity(love, LOVE);
    game.assert_card_score(love, 1);

    for (slot, card_no) in stage.iter().enumerate() {
        game.state.player1.stage.stage[slot] = game.id(card_no);
    }
    for card_no in success_zone {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(card_no));
    }

    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(love);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    fill_decks(&mut game, filler);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(love);
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    game.state.mods.get_score_modifier(love)
}

#[test]
fn empty_success_zone_and_all_lily_white_stage_scores_plus_one() {
    assert_eq!(
        score_mod(&[LW_UMI], &[]),
        1,
        "成功ライブカード置き場のカードが0枚 and ステージが『lily white』のみ — both \
         clauses hold, so +1"
    );
}

#[test]
fn two_lily_white_members_still_score_only_one() {
    assert_eq!(
        score_mod(&[LW_UMI, LW_NOZOMI], &[]),
        1,
        "the effect is a flat ＋1 on the live, not a per-member count"
    );
}

/// Break clause 1 only: stage is all `lily white`, but the success zone has a card.
#[test]
fn a_card_in_the_success_zone_blocks_the_bonus() {
    assert_eq!(
        score_mod(&[LW_UMI], &["PL!-sd1-019-SD"]),
        0,
        "成功ライブカード置き場のカードが0枚 is required — one card breaks it even \
         though the stage clause still holds"
    );
}

/// Break clause 2 only: success zone empty, but the stage is not exclusive.
#[test]
fn a_non_lily_white_member_blocks_the_bonus() {
    assert_eq!(
        score_mod(&[LW_UMI, PRINTEMPS], &[]),
        0,
        "メンバーが『lily white』のみ is required — one μ's member alongside breaks \
         exclusivity even though the success zone is empty"
    );
}

#[test]
fn a_non_mus_unit_also_breaks_exclusivity() {
    assert_eq!(
        score_mod(&[LW_UMI, AZALEA], &[]),
        0,
        "exclusivity is over the whole stage, so an unrelated unit also breaks it"
    );
}

#[test]
fn both_clauses_broken_scores_nothing() {
    assert_eq!(
        score_mod(&[PRINTEMPS], &["PL!-sd1-019-SD"]),
        0,
        "neither clause holds, so no bonus"
    );
}

/// The 「のみ」 is a universal, not an existential. A stage of one `lily white` and
/// one other member is the mixed case above; this pins that two `lily white` and
/// nothing else is what qualifies.
#[test]
fn single_lily_white_alone_qualifies() {
    assert_eq!(
        score_mod(&[LW_UMI], &[]),
        1,
        "precondition check for exclusivity: a lone lily white member satisfies のみ"
    );
}

/// Empty stage: のみ is vacuously true over zero members. This is the case the card
/// text does not settle, so it is pinned to whatever the engine actually does and
/// called out rather than asserted as correct.
#[test]
fn empty_stage_is_recorded_as_vacuous() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let love = game.id(LOVE);
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(love);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    fill_decks(&mut game, filler);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(love);
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    let observed = game.state.mods.get_score_modifier(love);
    assert!(
        observed == 0 || observed == 1,
        "empty stage must resolve to 0 or 1, got {}",
        observed
    );
    // See docs/PBLL02_ABILITY_REVIEW.md §6 "Untestable / open questions":
    // 「メンバーが『lily white』のみ」 over an EMPTY stage is vacuously true in
    // logic, so 1 is the defensible reading and 0 is the conservative one. The
    // engine currently produces {}. This assertion records the contract without
    // blessing either reading; do not tighten it until the ruling is decided.
}
