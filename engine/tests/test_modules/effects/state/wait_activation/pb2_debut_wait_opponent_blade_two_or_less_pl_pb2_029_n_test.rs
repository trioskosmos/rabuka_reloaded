//! 絢瀬絵里 `PL!-pb2-029-N` (登場 / ライブ開始時)
//!
//! 自分のステージにいるメンバーが『μ's』のみの場合、相手のステージにいる元々持つ
//! {{icon_blade.png|ブレード}}の数が2つ以下のメンバー1人をウェイトにする。
//! （ウェイト状態のメンバーが持つ{{icon_blade.png|ブレード}}は、エールで公開する枚数を
//! 増やさない。）
//!
//! The blade bound is the same mechanism as `PL!-pb2-006` (≤1) and `PL!-pb2-033` (≤3),
//! at a third value, so this is a boundary regression guard across the family: 2
//! qualifies, 3 does not. Two gates stack — stage exclusivity over μ's, and the bound
//! on the target — so each is broken alone below.
//!
//! ONLY ONE ally is ever staged. The stage has three slots and Maki takes the centre,
//! so staging two allies means playing into an occupied centre — a BATON TOUCH that
//! silently replaces the second ally. An earlier version of this file did that, and
//! the stage was never actually mixed, so the exclusivity negative passed for the
//! wrong reason.
//!
//! The parenthetical is a rules clarification about the yell, not an effect (docs §8.3).

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const MAKI: &str = "PL!-pb2-029-N";
/// Opponent members, by PRINTED blade count. Every fixture's blade is pinned in the
/// test, because a wrong fixture would make the bound pass or fail for the wrong reason.
const IN_1: &str = "PL!-PR-014-PR"; // blade 1
const IN_2: &str = "PL!-bp3-009-P"; // blade 2 — the boundary
const OUT_3: &str = "PL!N-bp7-016-N"; // blade 3
const OUT_4: &str = "PL!N-bp5-001-AR"; // blade 4
/// A μ's ally for our left slot.
const MUS_ALLY: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
/// A non-μ's ally for the same slot.
const NON_MUS_ALLY: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

fn fire_debut(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        // This card's trigger list is the COMBINED "ライブ開始時, 登場", so match on
        // containment rather than equality.
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref().is_some_and(|t| t.contains("登場")))
            .unwrap_or_else(|| panic!("{} lacks a 登場 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(cid);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
}

/// Debut MAKI with `ally` (if any) in our left slot and `theirs` on the opponent's
/// stage, answering any target prompt with the first option.
fn debut_with(ally: Option<&str>, theirs: &[&str]) -> (TestGame, Vec<i16>) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    if let Some(c) = ally {
        game.state.player1.stage.stage[MemberArea::LeftSide as usize] = game.id(c);
    }
    let ids: Vec<i16> = theirs.iter().map(|c| game.id(c)).collect();
    for (i, &id) in ids.iter().enumerate() {
        game.state.player2.stage.stage[i] = id;
    }
    let maki = game.id(MAKI);
    game.assert_card_identity(maki, MAKI);
    game.state.player1.hand.cards.push(maki);
    game.give_energy(30);
    game.play_to_stage(maki, MemberArea::Center);
    fire_debut(&mut game, maki);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    (game, ids)
}

fn waited(game: &TestGame, ids: &[i16]) -> Vec<bool> {
    ids.iter()
        .map(|&id| game.state.mods.get_orientation_modifier(id) == Some("wait"))
        .collect()
}

/// Pin the fixtures' printed blade counts — the whole test is about this number, and
/// pin them before the assertions that depend on them.
#[test]
fn fixture_blade_counts_are_what_the_test_assumes() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for (card_no, want) in [(IN_1, 1u8), (IN_2, 2), (OUT_3, 3), (OUT_4, 4)] {
        assert_eq!(
            game.db.get_card(game.id(card_no)).map(|c| c.blade),
            Some(want),
            "{card_no} must print {want} blade or the boundary test means nothing"
        );
    }
    assert_eq!(
        game.state.player1.stage.stage.len(),
        3,
        "the stage has three slots — Maki takes the centre, so at most one ally"
    );
}

#[test]
fn an_opponent_member_at_the_blade_bound_is_waited() {
    let (game, ids) = debut_with(Some(MUS_ALLY), &[IN_2]);
    assert_eq!(
        waited(&game, &ids),
        vec![true],
        "元々持つ{{icon_blade}}の数が2つ以下 — 2 is the boundary and qualifies"
    );
}

#[test]
fn a_one_blade_member_is_waited() {
    let (game, ids) = debut_with(Some(MUS_ALLY), &[IN_1]);
    assert_eq!(
        waited(&game, &ids),
        vec![true],
        "1つ — below the bound, so the target qualifies"
    );
}

#[test]
fn a_three_blade_member_is_never_offered() {
    let (game, ids) = debut_with(Some(MUS_ALLY), &[OUT_3]);
    assert_eq!(
        waited(&game, &ids),
        vec![false],
        "3つ — one over the bound, so the member is not an eligible target"
    );
}

#[test]
fn only_the_in_bound_member_is_affected() {
    let (game, ids) = debut_with(Some(MUS_ALLY), &[OUT_3, IN_2]);
    assert_eq!(
        waited(&game, &ids),
        vec![false, true],
        "of a 3-blade and a 2-blade member only the 2-blade one is waited"
    );
}

/// Break the exclusivity half only: the target is eligible, our stage is not all μ's.
#[test]
fn a_non_mus_ally_blocks_the_effect() {
    let (game, ids) = debut_with(Some(NON_MUS_ALLY), &[IN_2]);
    assert_eq!(
        waited(&game, &ids),
        vec![false],
        "メンバーが『μ's』のみ — with an AZALEA member beside her, the stage is not \
         μ's-only, so the effect must not fire even though the target qualifies"
    );
}

/// のみ is a universal, so Maki alone on the stage satisfies it.
#[test]
fn an_empty_ally_slot_still_satisfies_exclusivity() {
    let (game, ids) = debut_with(None, &[IN_2]);
    assert_eq!(
        waited(&game, &ids),
        vec![true],
        "のみ is vacuously true over a stage holding only her"
    );
}

#[test]
fn no_eligible_opponent_member_is_harmless() {
    let (game, ids) = debut_with(Some(MUS_ALLY), &[OUT_4]);
    assert_eq!(
        waited(&game, &ids),
        vec![false],
        "4つ — over the bound, so nothing is waited"
    );
}
