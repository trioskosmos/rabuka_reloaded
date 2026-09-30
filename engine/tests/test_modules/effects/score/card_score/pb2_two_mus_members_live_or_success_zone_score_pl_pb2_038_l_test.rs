//! 嵐のなかの恋だから `PL!-pb2-038-L` (常時)
//!
//! このカードが自分のライブカード置き場**か**成功ライブカード置き場にあり、
//! かつ自分のステージに2人の『μ's』のメンバーのみがいる場合、ライブの合計スコアを＋１する。
//! この効果は重複しない。
//!
//! Two zone branches joined by 「か」, a stage-count clause reading "exactly 2 μ's",
//! and a non-stackable clause. The zone branch is the one the parse was losing, so
//! both zones are exercised explicitly.
//!
//! Modelled on `success_zone_group_gated_constant_score_test.rs` (Angelic Angel),
//! which is the same shape — note its `live_set_zone` negative case is exactly the
//! half of this card's zone clause that must ALSO work positively here.

use crate::helpers::*;

const LOVE: &str = "PL!-pb2-038-L";
/// Two distinct μ's members with no abilities of their own, so the only thing that
/// can satisfy the stage clause is this card's own condition.
const MUS_A: &str = "PL!-bp6-017-N"; // 小泉花陽
const MUS_B: &str = "PL!-bp5-012-N"; // 南 ことり
/// A third μ's, for the "3人は満たさない" case.
const MUS_C: &str = "PL!-pb1-026-N";
/// Not μ's.
const NON_MUS: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

/// Put the live in `zone` with `members` on stage; return player 1`s constant total
fn score_mod_in(zone: &str, members: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let love = game.id(LOVE);
    game.assert_card_identity(love, LOVE);
    game.assert_card_score(love, 5);
    for (i, c) in members.iter().enumerate() {
        let id = game.id(c);
        if *c != NON_MUS {
            game.assert_card_in_group(id, "μ's", "stage fixture");
        }
        game.state.player1.stage.stage[i] = id;
    }
    match zone {
        "live" => game.state.player1.live_card_zone.cards.push(love),
        "success" => game.state.player1.success_live_card_zone.cards.push(love),
        _ => panic!("unknown zone {zone}"),
    }
    game.state.recalculate_constants();
    // This card targets ライブの合計スコア (target=live_total), so the bonus lands in
    // the player's constant total, NOT in the card's own score modifier.
    game.state.mods.p1_constant_total_score_bonus
}

/// ライブの合計スコアを＋１する on a live card in a live zone.
///
/// This needed two engine/parser changes: the success-zone constant evaluator had no
/// `live_total` case (it fell into "target the live set zone cards" and, with that
/// zone empty, silently scored nothing), and there was no constant scan of the live
/// card zone at all. The 「AかB」 also needed marking, because the multi-location path
/// otherwise reads a two-zone condition as the 「AをBに」 move shape and accepts only
/// the second zone.
#[test]
fn in_success_live_zone_with_two_mus_members_scores_plus_one() {
    assert_eq!(
        score_mod_in("success", &[MUS_A, MUS_B]),
        1,
        "成功ライブカード置き場にあり and 2人の『μ's』のメンバーのみ — both clauses hold"
    );
}

/// The 「か」 branch — ライブカード置き場 — is the half the parse used to drop, and for
/// which there was no constant scan at all.
#[test]
fn in_live_card_zone_with_two_mus_members_also_scores_plus_one() {
    assert_eq!(
        score_mod_in("live", &[MUS_A, MUS_B]),
        1,
        "ライブカード置き場**か**成功ライブカード置き場 — the live card zone is the \
         other half of the same disjunction, so it scores too"
    );
}

#[test]
fn one_member_is_not_enough() {
    assert_eq!(
        score_mod_in("success", &[MUS_A]),
        0,
        "2人の『μ's』のメンバーのみ — a single member does not satisfy 2人"
    );
}

#[test]
fn three_members_do_not_satisfy_exactly_two() {
    assert_eq!(
        score_mod_in("success", &[MUS_A, MUS_B, MUS_C]),
        0,
        "2人のみ is a count of exactly two, so three μ's members must NOT qualify"
    );
}

#[test]
fn a_non_mus_member_breaks_exclusivity() {
    assert_eq!(
        score_mod_in("success", &[MUS_A, NON_MUS]),
        0,
        "『μ's』のメンバーのみ — an AZALEA member alongside breaks exclusivity"
    );
}

/// The card is the 自分 in the zone clause, so it has to be in one of those zones.
#[test]
fn out_of_both_live_zones_grants_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let love = game.id(LOVE);
    game.state.player1.stage.stage[0] = game.id(MUS_A);
    game.state.player1.stage.stage[1] = game.id(MUS_B);
    // Deliberately in neither zone.
    game.state.player1.hand.cards.push(love);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "このカードが自分のライブカード置き場か成功ライブカード置き場にあり — a card in \
         neither zone is not the subject, so the constant does not apply"
    );
}

/// 非 stackable: この効果は重複しない.
#[test]
fn two_copies_do_not_stack() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.stage.stage[0] = game.id(MUS_A);
    game.state.player1.stage.stage[1] = game.id(MUS_B);
    let a = game.id(LOVE);
    game.state.player1.success_live_card_zone.cards.push(a);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "precondition: a single copy contributes +1 to ライブの合計スコア"
    );

    let b = game.new_id(LOVE);
    game.state.player1.success_live_card_zone.cards.push(b);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "この効果は重複しない — a second copy must not raise the total to 2"
    );
}

/// The configuration the previous test does not build: one copy in EACH zone.
///
/// 「この効果は重複しない」 is about the *ability*, not the zone, and the card's own zone
/// clause is a disjunction (「ライブカード置き場**か**成功ライブカード置き場」). One copy
/// performing now and a second already succeeded is a real board state, so the two copies sit
/// in different zones — the only arrangement where a de-dup keyed per-zone instead of
/// per-ability would quietly pay twice.
#[test]
fn one_copy_in_each_live_zone_still_scores_only_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    game.state.player1.stage.stage[0] = game.id(MUS_A);
    game.state.player1.stage.stage[1] = game.id(MUS_B);

    let performing = game.id(LOVE);
    game.assert_card_identity(performing, LOVE);
    let succeeded = game.new_id(LOVE);
    game.state.player1.live_card_zone.cards.push(performing);
    game.state.player1.success_live_card_zone.cards.push(succeeded);

    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "one copy in ライブカード置き場 and one in 成功ライブカード置き場 — both satisfy the \
         zone disjunction, but この効果は重複しない, so the total must stay at +1. A value of \
         2 means non_stackable is de-duplicating per zone rather than per ability."
    );
}
