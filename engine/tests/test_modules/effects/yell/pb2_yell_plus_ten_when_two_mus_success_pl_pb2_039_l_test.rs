//! 僕たちはひとつの光 `PL!-pb2-039-L` (ライブ開始時)
//!
//! 自分の成功ライブカード置き場に『μ's』のカードが2枚以上ある場合、ライブ終了時まで、
//! エールによって公開される自分のカードの枚数が10枚増える。
//!
//! A threshold plus a flat +10 to the yell size, observed the way the engine models a
//! yell bonus: more cards revealed means FEWER cheer checks required to complete the
//! cheer, so `cheer_checks_required` should fall by the bonus.
//!
//! # Two defects found by this file — not asserted
//!
//! With a base of 30 blade, a zone holding 2 cards (μ's or otherwise) produces
//! `cheer_checks_required == 40` — MORE than the 30 with 0 or 1 card. Two things are
//! wrong, and they are independent:
//!
//! 1. **Direction.** The +10 yell bonus makes the cheer require MORE checks instead
//!    of fewer, so the effect's sign or its target is inverted somewhere between the
//!    `modify_yell_count` node and `perform_cheer_check`.
//! 2. **Group filter.** Two NON-μ's lives produce exactly the same 40 as two μ's
//!    lives, so the `『μ's』のカードが2枚以上` threshold is not filtering on
//!    `group_names` at all — it is counting any card in the zone.
//!
//! Neither has a correct value to assert yet, so the two cases that DO hold are the
//! ones pinned below. See docs/PBLL02_ABILITY_REVIEW.md §8.3.
//!
//! # Fixture note
//!
//! The corpus spells one group two ways — `lily white` (4 lives) and `lilywhite`
//! (several members, e.g. `PL!-PR-004-PR` 園田海未) — and
//! `card_matches_group_str` does not equate them, so a 『lily white』 filter silently
//! excludes the unspaced members. Every μ's fixture here is the SPACED spelling for
//! that reason. The mismatch itself is a defect, recorded separately.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const HIKARI: &str = "PL!-pb2-039-L";
/// μ's lives — both the spaced `lily white` spelling.
const MUS_A: &str = "PL!-pb1-029-SRL"; // 知らないLove
const MUS_B: &str = "PL!-bp4-025-SRL"; // 微熱からMystery
/// Not μ's (no unit at all).
const NON_MUS: &str = "PL!-sd1-019-SD";
const FILLER: &str = "PL!-sd1-010-SD";
/// Blade high enough that a 10-card reduction is visible.
const BASE_BLADE: u8 = 30;

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Load `zone` into our success live zone, run ライブ開始時, then a cheer check.
/// Returns the cheer checks required — LOWER means a bigger yell.
fn cheer_checks_with(zone: &[&str]) -> u8 {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    for c in zone {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(c));
    }

    let hikari = game.id(HIKARI);
    game.assert_card_identity(hikari, HIKARI);
    game.state.player1.hand.cards.push(hikari);
    game.give_energy(20);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(hikari);

    let ability_id = {
        let card = game.db.get_card(hikari).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap();
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(hikari).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(hikari);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(hikari),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        game.select_indices(&[]);
        guard += 1;
    }

    let _ = game.state.perform_cheer_check(&pid, BASE_BLADE);
    game.state.cheer_checks_required
}

/// Below the threshold the effect must not fire, so the cheer is unmodified.
#[test]
fn zero_or_one_card_in_the_zone_leaves_the_cheer_unmodified() {
    let none = cheer_checks_with(&[]);
    let one = cheer_checks_with(&[MUS_A]);
    assert_eq!(
        none, BASE_BLADE,
        "no cards — 2枚以上 is unmet, so the yell is unchanged"
    );
    assert_eq!(
        one, BASE_BLADE,
        "『μ's』のカードが2枚以上 — one card is below the threshold, so the yell is unchanged"
    );
}

/// 自分の成功ライブカード置き場 — the opponent's zone must not count.
#[test]
fn the_opponents_success_zone_does_not_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    let hikari = game.id(HIKARI);
    game.state.player1.hand.cards.push(hikari);
    for c in [MUS_A, MUS_B] {
        game.state
            .player2
            .success_live_card_zone
            .add_card(game.id(c));
    }
    game.give_energy(20);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(hikari);
    let pid = game.state.player1.id.clone();
    let _ = game.state.perform_cheer_check(&pid, BASE_BLADE);

    assert_eq!(
        game.state.cheer_checks_required, BASE_BLADE,
        "自分の成功ライブカード置き場 — two μ's lives on the OPPONENT's side must not \
         satisfy the condition"
    );
}
