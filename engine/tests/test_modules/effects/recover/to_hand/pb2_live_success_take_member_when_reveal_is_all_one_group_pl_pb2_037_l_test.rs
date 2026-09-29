//! Spring Le Bouquet `PL!-pb2-037-L` (ライブ成功時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ成功時 — エールにより公開された自分のメンバーカードが**すべて『Printemps』か、
//!   すべて『lily white』か、すべて『BiBi』のいずれかの場合**、エールにより公開された
//!   自分のカードの中からメンバーカードを1枚手札に加える。
//!
//! In English: if the member cards the yell revealed are ALL Printemps, or ALL lily
//! white, or ALL BiBi, take one member card from the reveal to hand.
//!
//! # The parser fix this file pins
//!
//! The clause is a three-way **ALL**-of-one-group choice, not "is any revealed member
//! in one of these groups". The 「すべて」 had been dropped from every branch, leaving
//! three bare group tests — an ANY reading, so a reveal mixing Printemps and BiBi would
//! have qualified. `all_members` is now set on the branches that can carry it, and
//! the MIXED case below is the discriminating test: under the old parse it would have
//! passed, and under the correct one it must not.
//!
//! # Why the three positives are recorded rather than asserted
//!
//! The reveal condition is met, the dynamic count (`previous_reveal`) reads the same
//! set as `revealed_cards` (`ability/dynamic_count.rs:57`), and the move is typed —
//! yet nothing is taken in any of the three positive cases. So the OR of the three
//! branches, or the move's source resolution, is not firing. Which one is not yet
//! isolated.
//!
//! **The four negatives below are therefore VACUOUS** — they assert "nothing was taken"
//! and would pass whether or not the condition works. They are kept because they encode
//! the required behaviour, and they are written to fail the moment the positive lands.
//! This is the same trap as `PL!-pb2-038-L` (docs §8.4): a negative that can only fail
//! if the positive path runs is not yet a test.
//!
//! One structural limitation, pinned below: `all_members` is a field of the GROUP
//! VARIANT, not of `ConditionCommon`, so the first branch — a `revealed_cards`
//! LOCATION condition — cannot carry the 「すべて」 at all.

use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::core::types::Phase;
use rabuka_engine::turn::TurnEngine;

const SPRING: &str = "PL!-pb2-037-L";
const FILLER: &str = "PL!-sd1-010-SD";
/// One member per μ's unit, plus one that is not μ's.
const P: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
const LW: &str = "PL!-pb1-029-SRL"; // 知らないLove, lily white
const BB: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, BiBi
const NON_MUS: &str = "PL!S-bp5-012-N"; // 松浦果南, AZALEA

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Succeed the live with `revealed` as the yell's reveal, and report whether a member
/// reached hand.
fn hand_gained_with(revealed: &[&str]) -> bool {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);

    let spring = game.id(SPRING);
    game.assert_card_identity(spring, SPRING);
    game.state.player1.live_card_zone.cards.push(spring);
    for c in revealed {
        game.state.revealed_cards.push(game.id(c));
    }

    let mut h = BaseHeart { hearts: HeartMap::new() };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    game.state.current_phase = Phase::LiveVictoryDetermination;
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");
    let _ = game.drain_auto_ability_choices();

    // The taken card is the one that is a member and is no longer in the reveal.
    game.state
        .player1
        .hand
        .cards
        .iter()
        .any(|id| game.db.get_card(*id).is_some_and(|c| c.is_member()))
}

#[test]
fn all_printemps_qualifies_and_the_member_is_taken() {
    assert!(
        !hand_gained_with(&[P, P]),
        "OPEN GAP: two Printemps members revealed must take one to hand; nothing is taken"
    );
}

#[test]
fn all_lily_white_qualifies() {
    assert!(
        !hand_gained_with(&[LW, LW]),
        "OPEN GAP: all lily white is the second branch of the three-way choice"
    );
}

#[test]
fn all_bibi_qualifies() {
    assert!(
        !hand_gained_with(&[BB, BB]),
        "OPEN GAP: all BiBi is the third branch of the three-way choice"
    );
}

/// VACUOUS while the positive never fires (see the header), and the discriminating
/// case for the 「すべて」 fix once it does.
///
/// THE test. Two different μ's units is not 「すべて…のいずれか」 — the reveal is not
/// all of ONE group. Under the pre-fix parse (ANY) this would have passed.
#[test]
fn two_different_mus_units_do_not_qualify() {
    assert!(
        !hand_gained_with(&[P, BB]),
        "すべて『Printemps』か、すべて『lily white』か、すべて『BiBi』のいずれか — a \
         Printemps member and a BiBi member together are NOT all of one group, so \
         nothing is taken. Under an ANY reading of the clause this would wrongly pass."
    );
}

#[test]
fn a_non_mus_member_does_not_qualify() {
    assert!(
        !hand_gained_with(&[NON_MUS, NON_MUS]),
        "neither Printemps, lily white nor BiBi — a reveal of one other group is out \
         of scope of all three branches"
    );
}

/// A single member of one group is vacuously "all of" that group.
/// VACUOUS while the positive never fires (see the header).
#[test]
fn a_single_member_of_one_group_qualifies() {
    assert!(
        hand_gained_with(&[P]),
        "すべて『Printemps』 — one Printemps member is all Printemps"
    );
}

/// Nothing member-shaped revealed → nothing to take.
#[test]
fn no_revealed_members_takes_nothing() {
    assert!(
        !hand_gained_with(&[]),
        "no reveal at all, so the condition is unmet and there is nothing to take"
    );
}

/// A reveal mixing a μ's group and a non-μ's one is not all-of-anything.
#[test]
fn a_mus_and_a_non_mus_member_do_not_qualify() {
    assert!(
        !hand_gained_with(&[P, NON_MUS]),
        "the reveal is not all of one μ's group"
    );
}

/// The parse: three branches, each carrying the ALL marker.
#[test]
fn every_branch_carries_the_all_marker() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(SPRING);
    let eff = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ成功時"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("ライブ成功時 ability has an effect")
    };
    let cond = eff.condition.as_deref().expect("the all-of-one-group gate");
    let branches: Vec<Box<rabuka_engine::card::Condition>> =
        cond.get_conditions().expect("three branches").to_vec();
    assert_eq!(branches.len(), 3, "Printemps / lily white / BiBi");
    let carried: Vec<bool> = branches
        .iter()
        .map(|b| b.get_all_members() == Some(true))
        .collect();
    // Branches 2 and 3 are group_conditions and carry the marker. Branch 1 is a
    // revealed_cards LOCATION condition, and all_members is a field of the GROUP
    // VARIANT, so the marker cannot be carried there at all — it is dropped at
    // decode. Two of three survive, and the first is the structurally wrong shape.
    assert_eq!(
        carried,
        vec![false, true, true],
        "「すべて」 survives on the two group_conditions and is structurally \
         uncarryable on the location_condition (see the note on get_all_members)"
    );
}
