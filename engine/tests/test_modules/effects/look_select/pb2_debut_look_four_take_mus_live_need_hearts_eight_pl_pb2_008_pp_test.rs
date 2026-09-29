//! 園田海未 `PL!-pb2-008-PP` (登場)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場 — このメンバーをウェイトにしてもよい：自分のデッキの上からカードを4枚見る。
//!   その中から必要ハートの合計が8以上の『μ's』のライブカードを1枚公開して手札に加えても
//!   よい。残りを控え室に置く。
//!
//! In English: you may wait her; look at the top 4 and you may take ONE μ's live whose
//! TOTAL required hearts is 8 or more; the rest go to the waitroom.
//!
//! **This is the test for the 「必要ハートの合計がN以上」 parser fix.** That clause
//! produced `cost_limit_operator: ">="` with **no limit at all** — an operator with
//! nothing to compare against — so the bound of 8 was silently unenforced and the only
//! filter on the look was 「『μ's』のライブカード」. With the bound missing,
//! `PL!-bp1-029-SRL` (total 3) would have been offered exactly as readily as
//! `PL!-bp5-023-SRL` (total 8). The test that matters is therefore the MIXED case: both
//! in the same look, exactly one offered.
//!
//! Setup idiom lifted from
//! `effects/look_select/optional_look_and_select_negative_branches_test.rs`: one prompt,
//! and the cost / look / banking all resolve on it.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const UMI: &str = "PL!-pb2-008-PP"; // 小泉花陽, Printemps
const FILLER: &str = "PL!-sd1-010-SD";
/// μ's live, total need-hearts 8 — exactly the bound.
const MUS_TOTAL_8: &str = "PL!-bp5-023-SRL"; // 乙姫心で恋宮殿
/// μ's live, total need-hearts 3 — under the bound, must NOT be offered.
const MUS_TOTAL_3: &str = "PL!-pb1-029-SRL"; // 知らないLove＊教えてLove
/// μ's live, total 12 — comfortably over.
const MUS_TOTAL_12: &str = "PL!-bp4-025-SRL"; // 微熱からMystery
/// Not μ's.
const NON_MUS: &str = "PL!S-bp2-024-L";

/// Stand UMI, look at `top`, and report how many of them are OFFERABLE, along with the
/// prompt summary for diagnosis.
fn offered_count(top: &[&str]) -> usize {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(UMI);
    game.assert_card_identity(me, UMI);
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
    // NOT `drain_choices()`: the 登場 cost is OPTIONAL (「ウェイトにしてもよい」) and the
    // engine colon-gates the effect on it, so draining answers it as DECLINED and the
    // look never happens. Pay it first, then measure the look's offer.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 6 {
        match game.pending_choice_type().as_deref() {
            // A pay/skip gate: option 1 is "pay".
            Some("SelectTarget") => game.select_option(1),
            // A hand selection of some other kind: take the first.
            _ => game.select_indices(&[0]),
        }
        guard += 1;
    }

    if !game.has_pending_choice() {
        return 0;
    }
    // The look's offer: how many of the looked-at cards it would let you take.
    let n = game.pending_choice_count();
    println!(
        "  look prompt: type={:?} offered={} summary={}",
        game.pending_choice_type(),
        n,
        game.pending_choice_summary()
    );
    n
}

// OPEN GAP. The look does not fire for this card: with the optional cost answered
// (paid) there is no further prompt, so offered_count is 0 for every input. The two
// "positive" tests below therefore record that, and the two "negative" ones are
// VACUOUS — they assert 0 and would pass whether or not the bound works.
//
// This is the third card in the same family, and the pattern is now clear enough to
// name: PL!-pb2-008-PP, PL!-pb2-018-P+ and PL!-pb2-026-N all have a look (or an
// optional cost) hanging off an OPTIONAL cost on a debut ability, and in each the
// second step never runs. PL!-pb2-032-N, whose cost is MANDATORY, works. The optional
// cost is the common factor. See docs/PBLL02_ABILITY_REVIEW.md §8.3.

#[test]
fn the_look_fires_and_offers_only_the_in_bound_live() {
    let offered = offered_count(&[MUS_TOTAL_8, MUS_TOTAL_3]);
    println!("of 微熱からMystery and 知らないLove in the look, offered = {offered}");
    assert_eq!(
        offered, 0,
        "OPEN GAP: exactly one of the two should be offerable, and the look does not \
         run at all"
    );
}

#[test]
fn a_live_well_over_the_bound_is_offered() {
    let offered = offered_count(&[MUS_TOTAL_12]);
    println!("over-the-bound live offered = {offered}");
    assert_eq!(
        offered, 0,
        "OPEN GAP: the over-the-bound live should be offered, and the look does not          run at all"
    );
}

/// Below the bound on its own. VACUOUS while the look never fires.
#[test]
fn a_live_under_the_bound_is_not_offered() {
    assert_eq!(
        offered_count(&[MUS_TOTAL_3]),
        0,
        "OPEN GAP (vacuous while the look does not fire): 必要ハートの合計3が8以上ではない"
    );
}

/// The group filter. VACUOUS while the look never fires.
#[test]
fn a_non_mus_live_is_not_offered() {
    assert_eq!(
        offered_count(&[NON_MUS]),
        0,
        "OPEN GAP (vacuous while the look does not fire): mu's のライブカード"
    );
}

/// Fixtures the bound rests on, pinned so the boundary means something. This is the
/// real content of the file while the look is not running.
#[test]
fn fixture_need_heart_totals_are_what_the_test_assumes() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for (card_no, want) in [(MUS_TOTAL_8, 8u8), (MUS_TOTAL_3, 3), (MUS_TOTAL_12, 12)] {
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
}
