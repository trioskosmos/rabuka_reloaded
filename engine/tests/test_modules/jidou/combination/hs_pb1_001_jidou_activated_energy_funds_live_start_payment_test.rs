//! PL!HS-pb1-001 日野下花帆 — an **L4 shared-resource** pair (see
//! `docs/JIDOU_COMBINATION_WORK.md` §2). The jidou is not decoration: its
//! activation is what makes the sibling's 2E payment affordable at all.
//!
//!   * ab#0 自動 ターン2回 「自分のステージにほかの『スリーズブーケ』のメンバーが
//!     登場するたび、E支払ってもよい。そうした場合、エネルギーを2枚アクティブにする」
//!   * ab#1 ライブ開始時 「Eを2枚支払ってもよい：ライブ終了時まで、heart04+ブレード」
//!
//! ## The link no single-ability test can see
//!
//! ab#0 spends 1 active energy to convert 2 WAITED energy into ACTIVE energy.
//! ab#1 then spends 2 active energy. So ab#0 is a *feeder for ab#1's payment*:
//! the energy ab#1 pays with is exactly the energy ab#0 activated. The existing
//! jidou test (`jidou/debut_watch/ally_member_debut_pay_energy_or_center_ally_blade_test.rs`)
//! only ever DECLINES ab#0's payment and never touches ab#1, so it proves the
//! offer exists but never the sharing.
//!
//! The fixture is tuned so the two halves are genuinely coupled:
//!
//!   5 active + 2 waited energy; play the 4-cost スリーズブーケ ally → 1 active, 2 waited.
//!   * ab#0 accepted: pay 1E (1→0 active), activate 2 waited (→2 active).
//!     ab#1 can then pay its 2E exactly. Without ab#0 there is only 1 active
//!     energy, so ab#1's 2E is UNPAYABLE — that is the negative that proves the
//!     link and is only observable when both abilities are driven together.
//!
//! A single appearance fires ab#0 once, so this file is independent of the
//! ターン2回 re-scan-guard bug (Bug B, §6) — no second firing is needed here.

use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const HANA: &str = "PL!HS-pb1-001-R";
const CB_ALLY: &str = "PL!HS-sd1-012-SD"; // スリーズブーケ member, the 「ほかの」 trigger
const FILLER: &str = "PL!-sd1-010-SD";

fn active(game: &TestGame) -> u8 {
    game.state.player1.energy_zone.active_count()
}

/// Fire ab#1 (ライブ開始時) through the real queue and drain, answering the
/// optional 2E payment with `pay`.
fn fire_live_start_paying(game: &mut TestGame, hana: i16) {
    fire_trigger(game, hana, AbilityTrigger::LiveStart, "ライブ開始時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        match game.get_pending_choice() {
            // The 2E payment: option 1 = pay.
            Choice::SelectTarget { .. } => game.select_choice_option(1),
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            Choice::SelectCard { .. } => game.select_indices(&[]),
            _ => game.select_indices(&[]),
        }
    }
}

/// Build the tuned fixture: 花帆 in センター, energy zone = 5 active + 2 waited,
/// the スリーズブーケ ally in hand. The exact counts are the point — see the
/// module header — so they are asserted, not assumed.
fn fixture(game: &mut TestGame) -> (i16, i16) {
    let hana = game.id(HANA);
    game.assert_card_identity(hana, HANA);
    game.assert_card_cost(hana, 11);
    let ally = game.id(CB_ALLY);
    game.assert_card_in_group(ally, "スリーズブーケ", "the appearing ally is a 『スリーズブーケ』 member");
    let filler = game.id(FILLER);

    game.state.player1.hand.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.add_to_stage(MemberArea::Center, hana);
    game.add_to_hand(ally);

    // 7 energy: 5 active + 2 waited.
    let e = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..7 {
        game.state.player1.energy_zone.cards.push(e);
    }
    game.state.player1.energy_zone.set_active_count(5);
    assert_eq!(active(game), 5, "precondition: 5 active energy");

    (hana, ally)
}

/// Play the ally (cost 4) and answer ab#0's optional pay-E with `accept`.
/// Returns nothing; read energy via `active`.
fn play_ally_and_resolve_jidou(game: &mut TestGame, ally: i16, accept: bool) {
    game.play_to_stage(ally, MemberArea::LeftSide);
    // Drain ab#0's conditional_optional (pay 1E → activate 2). accept → pay.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectTarget { .. } => {
                game.select_choice_option(if accept { 1 } else { 0 })
            }
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            _ => game.select_indices(&[]),
        }
    }
}

// ====================================================================
// The L4 link: ab#0's activation is what lets ab#1 pay.
// ====================================================================

/// ACCEPT ab#0, then ab#1: the jidou pays 1E to activate the 2 waited energy,
/// and the sibling spends exactly those 2 activated energy to gain heart04+blade.
///
/// The link observable is the energy ledger: after the ally's 4-cost there is
/// 1 active energy, ab#0 converts the 2 waited → 2 active, and ab#1's 2E drops
/// the zone back to 0. No single-ability test observes that handoff.
#[test]
fn hs_pb1_001_jidou_activated_energy_is_exactly_what_live_start_pays() {
    let mut game = TestGame::new(load_real_database());
    let (hana, ally) = fixture(&mut game);

    play_ally_and_resolve_jidou(&mut game, ally, true);

    // 5 active − 4 (ally cost) = 1 active, then ab#0 paid 1E (→0) and
    // activated 2 waited (→2).
    assert_eq!(
        active(&game),
        2,
        "ab#0: after the 4-cost ally (5→1) it paid 1E (1→0) and activated the 2 \
         waited energy (0→2). The zone must hold 2 active for ab#1."
    );

    // ab#1 spends exactly those 2.
    fire_live_start_paying(&mut game, hana);
    assert_eq!(
        active(&game),
        0,
        "ab#1 paid its 2E out of the 2 energy ab#0 activated — the zone empties. \
         This is the link: the sibling's payment is funded by the jidou."
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(hana),
        1,
        "ab#1 granted its ブレード to 花帆 (exact, not `> 0`: the grant is a fixed \
         amount, and an absolute non-delta predicate is the shape \
         `jidou_test_audit.py --sweep d` exists to flag — the sibling \
         `hs_pb1_009` test was caught doing the same thing and fixed)"
    );
}

/// DECLINE ab#0, then ab#1: without the jidou's activation there is only 1 active
/// energy, so ab#1's 2E is unpayable and it must not grant.
///
/// This is the negative that only the combination can produce. Drop ab#0 and
/// ab#1's payment is unaffordable; drop ab#1 and the activation is just energy.
/// Neither single-ability test sees the coupling — this does.
#[test]
fn hs_pb1_001_declining_jidou_leaves_live_start_unable_to_pay() {
    let mut game = TestGame::new(load_real_database());
    let (hana, ally) = fixture(&mut game);

    play_ally_and_resolve_jidou(&mut game, ally, false); // DECLINE ab#0

    assert_eq!(
        active(&game),
        1,
        "ab#0 declined: the 4-cost ally left 1 active, and no waited energy was \
         activated. 1 active is the whole point — it is less than ab#1's 2E cost."
    );

    fire_live_start_paying(&mut game, hana); // attempt the 2E payment
    assert_eq!(
        active(&game),
        1,
        "ab#1's 2E is UNPAYABLE with only 1 active energy, so the payment cannot \
         deduct anything."
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(hana),
        0,
        "ab#1 could not pay, so it granted nothing. This is the proof the sibling's \
         grant is funded by the jidou, not independent of it."
    );
}
