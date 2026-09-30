//! 小泉花陽 `PL!-pb2-008-PP` (登場)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場 — このメンバーをウェイトにしてもよい：自分のデッキの上からカードを4枚見る。
//!   その中から必要ハートの合計が8以上の『μ's』のライブカードを1枚公開して手札に加えても
//! よい。残りを控え室に置く。
//!
//! In English: you may wait her; look at the top 4 and you may take ONE 『μ's』 live whose
//! **total** required hearts is 8 or more; the rest go to the 控え室.
//!
//! # The card is 小泉花陽, not 園田海未
//!
//! This file previously identified the card as 園田海未 in its header and its constant
//! comment. `cards.json` says `小泉花陽`. Both were wrong, and §8.11's annotation audit
//! exists precisely to catch that class of mistake.
//!
//! # The previous version of this file was measuring the offer by consuming it
//!
//! It recorded "OPEN GAP: the look does not run" and its helper returned 0 for every
//! input, including inputs where the look plainly fires. The reason is mechanical and
//! worth recording, because it is the exact shape of a vacuous test:
//!
//! ```text
//! while has_pending_choice() {          // prompt 1: pay/skip  -> select_option(1) = pay
//!     match type { SelectTarget => pay, // prompt 2: the LOOK   -> falls to `_`
//!                   _ => select_indices(&[0]) };            //     which TAKES a card
//! }
//! if !has_pending_choice() { return 0 }  // ...and then reports "0 offered"
//! ```
//!
//! So the test answered the very prompt it was supposed to measure, and reported the
//! absence of a prompt as the absence of an offer. Two of its four tests were labelled
//! VACUOUS in the source, which was honest, but the other two asserted `0` with an
//! OPEN GAP message and would have "passed" forever.
//!
//! The fix is one line of discipline: **read the offer before answering it.** The
//! pay/skip gate is `SelectTarget`; the look is `SelectCard` with `zone=looked_at`. Only
//! the second is the thing under test.
//!
//! # 合計 is a SUM, and the fixtures prove it
//!
//! `PL!-bp4-020-SRL` is the discriminator. Its required hearts are
//! heart01 x3 + heart03 x3 + heart0 x2 — total **8**, but no single colour reaches 8
//! (the largest is 3). Any per-colour reading of 「必要ハートの合計が8以上」 would reject it.
//! Any cost, score or blade misreading would too. It must be offered, and its presence
//! is what makes this a test of the specification rather than of the parser.
//!
//! Setup idiom from `effects/look_select/optional_look_and_select_negative_branches_test.rs`.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const HANAYOME: &str = "PL!-pb2-008-PP"; // 小泉花陽, Printemps
const FILLER: &str = "PL!-sd1-010-SD";
/// total need-hearts 8, but spread 3/3/2 so no single colour reaches 8.
const SUM_8_SPLIT: &str = "PL!-bp4-020-SRL"; // Love wing bell
/// total need-hearts 8 exactly, single colour.
const SUM_8_SOLID: &str = "PL!-bp5-023-SRL"; // 乙姫心で恋宮殿
/// total need-hearts 7 — the nearest miss.
const SUM_7: &str = "PL!-bp4-023-L"; // もぎゅっと"love"で接近中！
/// total need-hearts 3.
const SUM_3: &str = "PL!-bp4-026-L"; // ダイヤモンドプリンセスの憂鬱
/// Not 『μ's』.
const NON_MUS: &str = "PL!S-bp2-024-L";

/// Stand HANAYOME, look at `top`, and report how many of them the look OFFERS.
///
/// The offer is read from the look prompt itself, before it is answered. The optional
/// cost is paid, not declined: the engine colon-gates the effect on it, so declining
/// means the look never happens and the measurement is meaningless.
fn offered_count(top: &[&str]) -> usize {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(HANAYOME);
    game.assert_card_identity(me, HANAYOME);
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for card_no in top {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(card_no));
    }
    for _ in 0..40 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.add_to_hand(me);
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.play_to_stage(me, MemberArea::Center);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        match game.pending_choice_type().as_deref() {
            // The pay/skip gate. Option 1 is "pay", and paying is what lets the look run.
            Some("SelectTarget") => game.select_option(1),
            // The look. THIS is the prompt under test: read it, do not answer it.
            //
            // `pending_choice_option_count()`, NOT `pending_choice_count()`. The latter
            // is `max_select = min(requested, matching)` (`ability/effects/look.rs:110,156`),
            // so for a 「1枚」 take it is 1 whether one card or five qualify — the number
            // of cards you may PICK, not the number you may pick FROM. With it, this
            // helper returned 1 for any non-empty prompt and the bound assertions below
            // could not fail. The candidate count is `filtered_indices.len()`.
            Some("SelectCard") => return game.pending_choice_option_count().unwrap_or(0),
            _ => game.select_indices(&[0]),
        }
        guard += 1;
    }
    0
}

/// THE test. A card whose heart requirement is 8 only as a SUM is offerable.
///
/// 「必要ハートの合計が8以上」 — the bound is on the total, not on one colour. Every
/// misreading this test is designed to catch would reject this card: a per-colour bound
/// (its largest single colour is 3), a cost bound, a score bound, a blade bound.
#[test]
fn a_live_reaching_eight_only_in_total_is_offered() {
    assert_eq!(
        offered_count(&[SUM_8_SPLIT]),
        1,
        "PL!-bp4-020-SRL needs heart01 x3 + heart03 x3 + heart0 x2 = 8 in TOTAL, and \
         合計 is a sum — a per-colour reading would refuse it"
    );
}

/// The mixed case, and the one that actually pins the bound: exactly one of two
/// qualifying-by-different-reasons cards.
#[test]
fn the_bound_separates_the_seven_from_the_eight() {
    assert_eq!(
        offered_count(&[SUM_7, SUM_8_SOLID]),
        1,
        "もぎゅっと\"love\"で接近中！ totals 7 and 乙姫心で恋宮殿 totals 8, in the same look — \
         必要ハートの合計が8以上 admits exactly the second"
    );
}

/// Over the bound, and the group filter, both work.
#[test]
fn the_group_and_upper_bound_are_not_what_restricts_it() {
    assert_eq!(
        offered_count(&[SUM_8_SOLID]),
        1,
        "a single in-bound 『μ's』 live is offered"
    );
    assert_eq!(
        offered_count(&[NON_MUS]),
        0,
        "『μ's』のライブカード — a non-『μ's』 live is never offered, whatever its hearts"
    );
}

/// Below the bound alone. This one is NOT vacuous: the look demonstrably runs (the tests
/// above prove it on the same fixture and the same prompt), so a 0 here is a real
/// rejection rather than the absence of a prompt.
#[test]
fn a_live_under_the_bound_is_not_offered() {
    assert_eq!(
        offered_count(&[SUM_3]),
        0,
        "ダイヤモンドプリンセスの憂鬱 totals 3 — under 必要ハートの合計8, so not offerable. \
         The look is running here (see the tests above), so this 0 is a rejection, not a \
         missing prompt"
    );
}

/// The optional self-wait cost is real, and the effect is colon-gated on it.
///
/// 「このメンバーをウェイトにしてもよい」 is a `change_state` self-cost. Two things follow
/// and neither is checkable from the offer count: the member really does go to wait when
/// the cost is paid, and the look is skipped when it is declined.
#[test]
fn the_optional_self_wait_cost_is_what_unlocks_the_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(HANAYOME);
    let bound = game.new_id(SUM_8_SOLID);

    game.state.player1.hand.cards.clear();
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(bound);
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(game.new_id(FILLER));
    }
    game.add_to_hand(me);
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        None,
        "precondition: she is not waited before the play"
    );

    game.play_to_stage(me, MemberArea::Center);

    // Prompt 1 is the pay/skip gate; pay it.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "the optional cost is offered as a pay/skip gate"
    );
    game.select_option(1);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "このメンバーをウェイトにしてもよい — paying the cost puts HER into wait, which is \
         what the self-cost is for"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "and the effect only runs once the cost is paid — the colon-gate is on it"
    );
}

/// The fixtures the bound rests on, pinned so the boundary means something.
#[test]
fn fixture_need_heart_totals_are_what_the_test_assumes() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for (card_no, want) in [
        (SUM_8_SPLIT, 8u8),
        (SUM_8_SOLID, 8),
        (SUM_7, 7),
        (SUM_3, 3),
    ] {
        let total = game
            .db
            .get_card(game.id(card_no))
            .and_then(|c| c.need_heart.as_ref().map(|n| n.hearts.values_sum()))
            .unwrap_or(0);
        assert_eq!(
            total, want,
            "{card_no} must have total need-hearts {want} — the 8以上 bound is the point"
        );
    }

    // The discriminator's whole value is that no SINGLE colour reaches 8.
    let largest_colour = game
        .db
        .get_card(game.id(SUM_8_SPLIT))
        .and_then(|c| c.need_heart.as_ref().map(|n| n.hearts.values().copied().max().unwrap_or(0)))
        .unwrap_or(0);
    assert!(
        largest_colour < 8,
        "PL!-bp4-020-SRL is only useful here because no single colour reaches 8; its \
         largest is {largest_colour}"
    );
    game.assert_all_in_group(
        &[game.id(SUM_8_SPLIT), game.id(SUM_8_SOLID), game.id(SUM_7), game.id(SUM_3)],
        "μ's",
        "every candidate must be in the group the clause names, or the group filter \
         rather than the bound is what is being measured"
    );
}
