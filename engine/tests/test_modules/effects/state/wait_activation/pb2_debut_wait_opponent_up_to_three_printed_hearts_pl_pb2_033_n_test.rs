//! 西木野真姫 `PL!-pb2-033-N` (登場 / ライブ開始時)
//!
//! 相手のステージにいる元々持つハートの数が3つ以下のメンバー1人をウェイトにする。
//! （ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を
//! 増やさない。）
//!
//! The bound here is 3, not the 1 that `PL!-pb2-006` uses, so this file doubles as the
//! boundary case for the printed-heart ceiling: 3 qualifies, 4 does not. The
//! parenthetical is a rules clarification about the yell, not an effect, and is not
//! asserted (see docs/PBLL02_ABILITY_REVIEW.md §8.3).

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MAKI: &str = "PL!-pb2-033-N";
/// 3 printed hearts — the inclusive boundary.
const OPP_3: &str = "PL!S-bp5-005-AR";
/// 4 printed hearts — one over the ceiling.
const OPP_4: &str = "PL!-bp3-002-P";
/// 2 printed hearts.
const OPP_2: &str = "PL!-bp6-017-N";
/// 0 printed hearts.
const OPP_0: &str = "PL!-bp3-012-N";

/// Put the opponent members on their stage, then debut Maki. Returns
/// (maki, opponent ids) in the order given.
fn debut_against(opponent: &[&str]) -> (TestGame, i16, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let ids: Vec<i16> = opponent.iter().map(|c| game.id(c)).collect();
    for (i, &id) in ids.iter().enumerate() {
        game.state.player2.stage.stage[i] = id;
    }
    let maki = game.id(MAKI);
    game.assert_card_identity(maki, MAKI);
    game.state.player1.hand.cards.push(maki);
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    game.drain_auto_ability_choices();
    (game, maki, ids)
}

fn waited(game: &TestGame, ids: &[i16]) -> Vec<bool> {
    ids.iter()
        .map(|&id| game.state.mods.get_orientation_modifier(id) == Some("wait"))
        .collect()
}

#[test]
fn three_printed_hearts_is_waited() {
    let (game, _maki, ids) = debut_against(&[OPP_3]);
    assert_eq!(
        waited(&game, &ids),
        vec![true],
        "元々持つハートの数が3つ以下 — 3 is the boundary and must qualify"
    );
}

#[test]
fn four_printed_hearts_is_never_offered() {
    let (game, _maki, ids) = debut_against(&[OPP_4]);
    assert_eq!(
        waited(&game, &ids),
        vec![false],
        "4つ — one over the ceiling, so the member is not an eligible target"
    );
}

#[test]
fn two_and_zero_printed_hearts_are_waited() {
    let (game, _maki, ids) = debut_against(&[OPP_2]);
    assert_eq!(
        waited(&game, &ids),
        vec![true],
        "2つ以下 qualifies"
    );

    let (game, _maki, ids) = debut_against(&[OPP_0]);
    assert_eq!(
        waited(&game, &ids),
        vec![true],
        "0つ以下 qualifies — a member with no printed hearts still counts as 0"
    );
}

#[test]
fn only_the_qualifying_member_is_affected() {
    let (game, _maki, ids) = debut_against(&[OPP_4, OPP_3]);
    assert_eq!(
        waited(&game, &ids),
        vec![false, true],
        "of a 4-heart and a 3-heart member only the 3-heart one is waited"
    );
}

#[test]
fn one_of_several_eligible_is_chosen() {
    let (mut game, _maki, ids) = debut_against(&[OPP_3, OPP_2]);
    // Two members are eligible, so the target is not the engine's to resolve: the
    // prompt must really be offered. The old `if has_pending_choice()` skipped the
    // type and count assertions whenever it was absent, so an engine that picked
    // one on its own passed without ever proving メンバー1人 was asked for.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "two eligible members and メンバー1人 — the player picks one"
    );
    assert_eq!(
        game.pending_choice_count(),
        1,
        "メンバー1人 — exactly one, not both"
    );
    game.drain_choices_strict(&["SelectCard"], &[0]);
    assert_eq!(
        waited(&game, &ids).iter().filter(|&&w| w).count(),
        1,
        "precisely one opponent member ends up waited"
    );
}

#[test]
fn no_eligible_opponent_member_is_harmless() {
    let (game, _maki, ids) = debut_against(&[OPP_4]);
    assert_eq!(
        waited(&game, &ids),
        vec![false],
        "only a 4-heart member is on the opponent's stage, so nothing is waited"
    );
}

/// 相手のステージ — our own low-heart members are never targets.
#[test]
fn our_own_low_heart_member_is_not_a_target() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let maki = game.id(MAKI);
    let ally = game.id(OPP_2);
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = ally;
    game.state.player1.hand.cards.push(maki);
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    game.drain_auto_ability_choices();

    assert_eq!(
        game.state.mods.get_orientation_modifier(ally),
        None,
        "相手のステージ — our own 2-heart member is not an eligible target"
    );
}
