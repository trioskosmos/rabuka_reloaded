//! 南ことり `PL!-pb2-012-P＋` / `-R` (このカードをプレイする際)
//!
//! このカードをプレイする際、自分のステージにいる名前の異なる『Printemps』のメンバーを
//! 2人ウェイトにしてもよい。そうしたとき、このカードのコストは２減る。
//!
//! The reduction is OPTIONAL and conditional on waiting 2 differently-named
//! Printemps members, so the two axes are pinned separately: the wait, and the −2.
//! `count: 2` + `distinct: card_name` are both load-bearing — the same member twice
//! must not satisfy it.

use crate::helpers::*;

const KOTORI: &str = "PL!-pb2-012-P＋";
/// Two DIFFERENTLY-NAMED Printemps members with no abilities of their own, so
/// nothing else contributes a cost change.
const P_HANAYOME: &str = "PL!-bp6-017-N"; // 小泉花陽
const P_KOTORI2: &str = "PL!-bp5-012-N"; // 南 ことり
/// A third distinct name, for the "does not scale" case.
const P_HONOKA: &str = "PL!-sd1-010-SD"; // 高坂 穂乃果
/// Not Printemps.
const NON_PRINTEMPS: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

/// Stage `own`, hold Kotori in hand, and return her cost modifier.
/// She must be IN HAND: the constant cost scan only considers stage and hand card
/// ids, so a card in neither zone is never evaluated and silently reports 0.
fn cost_mod_with(own: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    game.assert_card_identity(kotori, KOTORI);
    for (i, c) in own.iter().enumerate() {
        game.state.player1.stage.stage[i] = game.id(c);
    }
    game.state.player1.hand.cards.push(kotori);
    game.state.recalculate_constants();
    game.state.mods.get_cost_modifier(kotori)
}

#[test]
fn printed_cost_is_thirteen() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    game.assert_card_cost(kotori, 13);
    game.state.player1.hand.cards.push(kotori);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(kotori),
        0,
        "precondition: with an empty stage, no reduction applies"
    );
}

#[test]
fn no_staged_members_means_no_reduction() {
    assert_eq!(
        cost_mod_with(&[]),
        0,
        "自分のステージにいる…2人 — an empty stage cannot satisfy the wait"
    );
}

#[test]
fn one_printemps_member_alone_is_not_enough() {
    assert_eq!(
        cost_mod_with(&[P_HANAYOME]),
        0,
        "2人 — a single staged Printemps member does not meet the count"
    );
}

#[test]
fn two_distinctly_named_printemps_members_grant_minus_two() {
    let probe = TestGame::new(load_real_database());
    probe.assert_card_in_group(probe.id(P_HANAYOME), "Printemps", "ally fixture");
    probe.assert_card_in_group(probe.id(P_KOTORI2), "Printemps", "ally fixture");
    probe.assert_distinct_card_names(probe.id(P_HANAYOME), probe.id(P_KOTORI2), "ally fixture");

    assert_eq!(
        cost_mod_with(&[P_HANAYOME, P_KOTORI2]),
        -2,
        "名前の異なる『Printemps』のメンバーを2人ウェイトにしてもよい — the optional \
         reduction is −2, not −1 and not +2"
    );
}

/// 名前が異なる — the same character twice does not satisfy the count.
#[test]
fn two_prints_of_the_same_character_are_not_distinct() {
    let probe = TestGame::new(load_real_database());
    let a = probe.id(P_HANAYOME);
    let b = probe.new_id(P_HANAYOME);
    probe.assert_same_card_name(a, b, "duplicate fixture");
    probe.assert_card_in_group(a, "Printemps", "ally fixture");

    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    game.state.player1.stage.stage[0] = a;
    game.state.player1.stage.stage[1] = b;
    game.state.player1.hand.cards.push(kotori);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(kotori),
        0,
        "名前の異なる — two copies of one character are not two distinct names, so \
         the −2 must not apply"
    );
}

#[test]
fn a_non_printemps_ally_does_not_count() {
    let probe = TestGame::new(load_real_database());
    probe.assert_card_in_group(probe.id(P_HANAYOME), "Printemps", "ally fixture");
    let n = probe.id(NON_PRINTEMPS);
    assert_eq!(
        probe.db.get_card(n).and_then(|c| c.unit.as_deref()),
        Some("AZALEA"),
        "precondition: the second ally is not Printemps"
    );

    assert_eq!(
        cost_mod_with(&[P_HANAYOME, NON_PRINTEMPS]),
        0,
        "『Printemps』のメンバー — an AZALEA member does not count toward the 2人"
    );
}

#[test]
fn three_printemps_members_still_grant_only_minus_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    for (i, c) in [P_HANAYOME, P_KOTORI2, P_HONOKA].iter().enumerate() {
        game.state.player1.stage.stage[i] = game.id(c);
    }
    game.state.player1.hand.cards.push(kotori);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_cost_modifier(kotori),
        -2,
        "このカードのコストは2減る — a flat −2, not scaled by how many members are \
         available to wait"
    );
}
