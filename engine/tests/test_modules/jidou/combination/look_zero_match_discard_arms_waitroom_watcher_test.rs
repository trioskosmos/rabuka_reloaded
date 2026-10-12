//! 園田海未 (PL!-sd1-004-SD) 登場 × 葉月 恋 (PL!SP-bp5-005-R＋) 自動.
//!
//! 海未の登場: 「自分のデッキの上からカードを5枚見る。その中から
//! 『μ's』のライブカードを1枚公開して手札に加えてもよい。残りを
//! 控え室に置く。」— a FILTER look-and-select with NO optional cost,
//! so a zero-match discard is the FIRST waitroom placement of the
//! turn (恋の自動 is ターン1回, and staging 恋 fires nothing: she has
//! no debut).
//!
//! 恋の自動 (ab#1): 「自分のメインフェイズの間、自分のカードが1枚
//! 以上いずれかの領域から控え室に置かれるたび、E支払ってもよい。
//! そうした場合、それらのカードの中から1枚手札に加える。」
//!
//! ## The link
//!
//! The zero-match branch of `offer_looked_at_selection`
//! (`ability/effects/look.rs`) used to push the unmatched pool
//! straight into the waitroom with `player.waitroom.add_card`,
//! which records NO movement event — so 恋の自動 never saw the
//! placement and never fired. The normal selection path
//! (`ability/effects/move/handlers.rs`) tracks the 「残りを控え室に
//! 置く」 discard with `finalize_card_movement`, which pushes the
//! movement event that arms the 自動. The zero-match branch now
//! routes the pool through the same `place_card_in_zone` +
//! `finalize_card_movement` pair, so the 自動 fires on it too.
//!
//! `sp_bp5_005_mill_arms_jidou_recover_test.rs` pins the same 自動
//! on the 起動's own mill; this file pins it on the look-and-select
//! zero-match discard, the path that was missing the movement event.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const REN: &str = "PL!SP-bp5-005-R＋";
const UMI: &str = "PL!-sd1-004-SD";
/// 松浦果南 (AZALEA) — an 『Aqours』 card and therefore outside
/// 『μ's』, so she is not a 『μ's』 live card and cannot satisfy
/// 海未's select filter. Five of her on the deck top make the look
/// a guaranteed zero-match.
const FILLER: &str = "PL!S-sd1-003-SD";

#[test]
fn zero_match_look_discard_arms_the_any_zone_waitroom_watcher() {
    let mut game = TestGame::new(load_real_database());
    let ren = game.id(REN);
    let umi = game.id(UMI);
    game.assert_card_identity(ren, REN);
    game.assert_card_identity(umi, UMI);

    // Five DISTINCT fillers, each an 『Aqours』 card and therefore
    // outside 『μ's』 — the look cannot match a 『μ's』 live card.
    let fillers: Vec<i16> = (0..5).map(|_| game.id(FILLER)).collect();
    for &f in &fillers {
        assert!(
            rabuka_engine::ability::util::card_matches_group_str(
                &game.db,
                f,
                Some("Aqours")
            ),
            "precondition: each filler must be an 『Aqours』 card, \
             hence outside 『μ's』"
        );
    }

    // 恋 on stage arms her 自動. She has no debut, so staging her
    // fires nothing and the zero-match discard below is the first
    // waitroom placement of the turn.
    game.add_to_stage(MemberArea::Center, ren);

    // Deck top = the five fillers: the look finds no 『μ's』 live card.
    game.state.player1.main_deck.cards.clear();
    for &f in &fillers {
        game.state.player1.main_deck.cards.push(f);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fillers[0]);
    }
    game.give_energy(15);

    // Play 海未 to an empty area: her 登場 fires, looks at 5, matches
    // nothing, and banks all five in the waitroom.
    game.add_to_hand(umi);
    game.play_to_stage(umi, MemberArea::LeftSide);

    // The five looked-at cards are in the waitroom.
    for &f in &fillers {
        assert!(
            game.state.player1.waitroom.cards.contains(&f),
            "the zero-match discard must bank the looked-at cards in 控え室; \
             waitroom is {:?}",
            game.state.player1.waitroom.cards
        );
    }

    // 恋の自動 must fire on that placement: the pay-E gate is offered.
    assert!(
        game.has_pending_choice(),
        "the zero-match discard placed 5 cards in 控え室, so 恋の自動 \
         (each_time any-zone→waitroom) must offer its pay-E gate; got {:?}",
        game.pending_choice_summary()
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "恋の自動 offers a pay-E SelectTarget gate"
    );

    // Decline the E: nothing is recovered, the five stay in the waitroom.
    game.select_choice_option(0);
    game.drain_choices_strict(&[], &[]);

    let hand: Vec<i16> = game.state.player1.hand.cards.to_vec();
    assert!(
        hand.iter().all(|c| !fillers.contains(c)),
        "declining the E recovers nothing: hand is {:?}",
        hand
    );
    for &f in &fillers {
        assert!(
            game.state.player1.waitroom.cards.contains(&f),
            "the looked-at cards stay in 控え室 after declining"
        );
    }
}
