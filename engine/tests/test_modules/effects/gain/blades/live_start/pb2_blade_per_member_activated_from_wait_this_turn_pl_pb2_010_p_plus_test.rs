//! 高坂穂乃果 `PL!-pb2-010-P＋` (ライブ開始時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ開始時 — ライブ終了時まで、自分のステージにいる、**このターン中に自分の
//!   『Printemps』のカードの効果によってウェイト状態からアクティブにしていたメンバー**
//!   1人につき、{{icon_blade.png|ブレード}}を得る。
//!
//! In English: +1 blade per member that one of MY Printemps cards' effects switched out
//! of wait and back to active earlier this turn, until the end of the live.
//!
//! # Why this needed a new engine record
//!
//! The clause is a per-turn HISTORY, not a state. It used to parse as a per-unit count
//! with `state: "wait"` — "members currently in wait" — which is a different set of
//! cards entirely, and is why this ability always granted nothing.
//!
//! Two pieces went in:
//!   * `GameState::cards_activated_this_turn` — the members switched out of wait this
//!     turn, recorded where the orientation actually changes
//!     (`ability/effects/change_state.rs`) and turned over with the other per-turn
//!     trackers in `clear_card_movement_tracking`.
//!   * `EffectFilter::activated_this_turn`, so the count reads that history instead of
//!     the current wait set.
//!
//! The history is a SET, not a tally: activating a member twice in one turn must not
//! make it count twice.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const ERI: &str = "PL!-pb2-010-P＋"; // 高坂穂乃果, Printemps
/// A Printemps member used as the victim of the activation.
const P_VICTIM: &str = "PL!-bp6-017-N"; // 小泉花陽
/// A non-Printemps member — the clause is 「自分の『Printemps』のカードの効果によって」.
const NON_P: &str = "PL!S-bp5-012-N"; // 松浦果南
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(f);
    }
}

fn fire_live_start(game: &mut TestGame, cid: i16) {
    let ability_id = {
        let card = game.db.get_card(cid).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("{} lacks a ライブ開始時 ability", card.card_no));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(cid).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(cid);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(cid),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();
}

/// Simulate "a Printemps card's effect switched this member out of wait", which is
/// what the engine records.
/// `TestGame::id` allocates a FRESH instance on every call, so an id must be
/// resolved once and reused — otherwise "the same member twice" records two different
/// cards and the set semantics look broken.
fn record_activation(game: &mut TestGame, id: i16) {
    if !game.state.cards_activated_this_turn.contains(&id) {
        game.state.cards_activated_this_turn.push(id);
    }
}

/// Stand ERI, put `history` into the activation history, fire ライブ開始時, and return
/// her blade count.
/// Blades GAINED by the history, not the absolute count: ERI prints a blade of her
/// own, so only the difference is the thing this card controls.
fn blade_with_history(history: &[&str]) -> i16 {
    blade_with_history_inner(history) - blade_with_history_inner(&[])
}

fn blade_with_history_inner(history: &[&str]) -> i16 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    let eri = game.id(ERI);
    game.assert_card_identity(eri, ERI);
    for c in history {
        let id = game.id(c);
        record_activation(&mut game, id);
    }
    fire_live_start(&mut game, eri);
    game.state.mods.get_blade_modifier(eri)
}

#[test]
fn an_activated_printemps_member_gives_one_blade() {
    assert_eq!(
        blade_with_history(&[P_VICTIM]),
        1,
        "このターン中に…ウェイト状態からアクティブにしていたメンバー1人につき — one \
         Printemps member in the activation history is +1 blade"
    );
}

#[test]
fn two_activated_printemps_members_give_two_blades() {
    assert_eq!(
        blade_with_history(&[P_VICTIM, P_NON_P_ALLY]),
        2,
        "1人につき scales with the number of members in the history"
    );
}

/// The group: 「自分の『Printemps』のカードの効果によって」 — a non-Printemps member in
/// the history does not count.
#[test]
fn a_non_printemps_activated_member_does_not_count() {
    assert_eq!(
        blade_with_history(&[NON_P]),
        0,
        "『Printemps』のカードの効果によって — an AZALEA member is not activated by a \
         Printemps card, so it does not count"
    );
}

#[test]
fn an_empty_history_gives_nothing() {
    assert_eq!(
        blade_with_history(&[]),
        0,
        "no member was switched out of wait this turn"
    );
}

/// The history is a SET: the same member twice is one member.
#[test]
fn the_same_member_twice_counts_once() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    let eri = game.id(ERI);
    let base = game.state.mods.get_blade_modifier(eri);
    let victim = game.id(P_VICTIM);
    record_activation(&mut game, victim);
    record_activation(&mut game, victim);
    assert_eq!(
        game.state.cards_activated_this_turn.len(),
        1,
        "the activation history is a set, not a tally"
    );
    fire_live_start(&mut game, eri);
    assert_eq!(
        game.state.mods.get_blade_modifier(eri) - base,
        1,
        "1人につき — a member activated twice in one turn is still one member"
    );
}

/// A member that is IN WAIT is not what the clause counts. This is the distinction the
/// old `state: "wait"` reading got wrong.
#[test]
fn a_member_currently_in_wait_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    let eri = game.id(ERI);
    let waiting = game.id(P_VICTIM);
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = waiting;
    game.state.mods.add_orientation_modifier(waiting, "wait");
    assert!(
        game.state.cards_activated_this_turn.is_empty(),
        "precondition: a member still in wait has not been activated"
    );
    let base = game.state.mods.get_blade_modifier(eri);
    fire_live_start(&mut game, eri);
    assert_eq!(
        game.state.mods.get_blade_modifier(eri) - base,
        0,
        "ウェイト状態からアクティブにしていた — a member that is STILL in wait has not \
         been switched out, and counting it was the original bug"
    );
}

/// The parse is a per-turn HISTORY marker, not a state filter.
#[test]
fn the_clause_parses_as_a_history_not_a_state() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(ERI);
    let eff = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("ライブ開始時 ability has an effect")
    };
    assert_eq!(
        eff.activated_this_turn_any(),
        Some(true),
        "このターン中に…アクティブにしていた — must be tagged as an activation history"
    );
    assert_eq!(
        eff.state_any(),
        None,
        "and must NOT also demand the member be in wait: the members are active now, \
         so `state: wait` would make the clause unsatisfiable"
    );
}

/// A second Printemps member for the two-member case.
const P_NON_P_ALLY: &str = "PL!-bp5-012-N"; // 南 ことり
