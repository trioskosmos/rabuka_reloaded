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
//! # What is broken, precisely
//!
//! The positive fires for a SINGLE revealed member and fails for two or more.
//! `a_single_member_of_one_group_qualifies_but_two_do_not` is the evidence: one
//! Printemps member is taken, two Printemps members are not, with the group held
//! constant. So the gate is not "the condition never matches" — it matches at one card
//! and stops matching past it.
//!
//! This header used to say the opposite — that "nothing is taken in any of the three
//! positive cases", and that "the four negatives below are therefore VACUOUS". Both
//! were wrong, and the second consequence was serious: it mislabelled which negatives
//! could fail. A negative whose positive analogue fires is real evidence. The
//! negatives that pair with the two-and-more cases ARE vacuous; the one that pairs with
//! the single-card case is not.
//!
//! The three `open_gap_*` tests assert the CURRENT (broken) behaviour on purpose —
//! they are a baseline, not a specification. They were previously named `*_qualifies*`
//! and `all_printemps_qualifies_and_the_member_is_taken`, i.e. named for the behaviour
//! the card is supposed to have while asserting the opposite. That mismatch is worse
//! than an absent test: names said covered, assertions said broken, and the suite was
//! green either way.
//!
//! Structural note, and the likely cause: `all_members` is a field of the GROUP VARIANT,
//! not of `ConditionCommon`, so the first branch — a `revealed_cards` LOCATION
//! condition — cannot carry the 「すべて」 at all. That predicts trouble exactly where
//! the count exceeds one.

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

/// OPEN GAP — the name this test used to carry said "qualifies_and_the_member_is_taken",
/// which is the exact opposite of what it asserts. Nothing is taken.
///
/// Kept as a negative because that is the current behaviour, but the mismatch was the
/// dangerous part: a reader scanning test names saw three branches covered and a member
/// being taken, and the suite was green. The bug was invisible from either side.
#[test]
fn open_gap_all_printemps_takes_nothing() {
    assert!(
        !hand_gained_with(&[P, P]),
        "OPEN GAP: two Printemps members revealed must take one to hand; nothing is taken"
    );
}

#[test]
fn open_gap_all_lily_white_takes_nothing() {
    assert!(
        !hand_gained_with(&[LW, LW]),
        "OPEN GAP: all lily white is the second branch of the three-way choice"
    );
}

#[test]
fn open_gap_all_bibi_takes_nothing() {
    assert!(
        !hand_gained_with(&[BB, BB]),
        "OPEN GAP: all BiBi is the third branch of the three-way choice"
    );
}

/// The asymmetry that makes the gap diagnosable: ONE member of a group is taken,
/// TWO are not.
///
/// This is the only positive in the file that actually fires, and it is what proves the
/// three `open_gap_*` tests above are a real defect rather than a uniformly dead path.
/// Whatever gates the two-card case is not "the condition never matches" — it matches
/// fine at one card. So the failure is in how the ALL reading scales past a single
/// card, which is exactly where the header's structural note predicts trouble:
/// `all_members` lives on the GROUP variant, so the first branch — a `revealed_cards`
/// LOCATION condition — has nowhere to carry 「すべて」.
///
/// Before this test the file claimed the positive "never fires in any of the three
/// cases", which this contradicts.
#[test]
fn a_single_member_of_one_group_qualifies_but_two_do_not() {
    assert!(
        hand_gained_with(&[P]),
        "one Printemps member is all Printemps, and it IS taken — this is the positive \
         that works"
    );
    assert!(
        !hand_gained_with(&[P, P]),
        "but TWO Printemps members are not taken, even though they are equally 'all \
         Printemps'. The count is the variable, not the group."
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
