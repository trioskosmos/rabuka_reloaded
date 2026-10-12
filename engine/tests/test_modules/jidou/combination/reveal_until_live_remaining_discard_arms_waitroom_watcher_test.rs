//! ミア・テイラー (PL!N-bp1-011-P) 登場.
//!
//! ミアの登場: 「手札を1枚控え室に置いてもよい：ライブカードが
//! 公開されるまで、自分のデッキの一番上のカードを公開し続ける。
//! そのライブカードを手札に加え、これにより公開されたほかの
//! すべてのカードを控え室に置く。」
//!
//! The cost is an optional `move_cards` from hand (rules 9.7.3.1.1:
//! declining it means the ability does not play), so the test PAYS
//! it with a spare hand card. The effect is `sequential`:
//!   1. `reveal_until_live_card` from deck_top — banks every card
//!      revealed (fillers + the live card) into `looked_at_cards`.
//!   2. `move_cards` source=`looked_at` → hand, card_type=live_card —
//!      pulls the live card out of the pool into hand.
//!   3. `move_cards` source=`looked_at_remaining` → discard — the
//!      fillers. This is the path under test: `resolve_from_looked_at`
//!      (`ability/effects/move/source.rs`) banks the pool with a bare
//!      `player.waitroom.add_card`, and the caller's `place_taken_cards`
//!      then re-places the SAME cards through `place_card_in_zone`
//!      (`Zone::Discard` → `waitroom.add_card`), so each filler lands
//!      in 控え室 TWICE.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MIA: &str = "PL!N-bp1-011-P";
/// Snow halation — a ライブ card with no ability, so
/// `reveal_until_live_card` stops here.
const LIVE: &str = "PL!-bp3-020-L";
/// 松浦果南 — a メンバー card (『Aqours』), so not a ライブ card;
/// `reveal_until_live_card` reveals past her. Two distinct
/// instances make the deck-top reveal deterministic.
const FILLER: &str = "PL!S-sd1-003-SD";
/// 園田海未 — a second, distinct member card used to PAY the
/// optional debut cost (「手札を1枚控え室に置いてもよい」).
const PAYER: &str = "PL!-sd1-004-SD";

#[test]
fn reveal_until_live_remaining_discard_banks_each_filler_once() {
    let mut game = TestGame::new(load_real_database());
    let mia = game.id(MIA);
    let live = game.id(LIVE);
    let payer = game.id(PAYER);
    game.assert_card_identity(mia, MIA);
    game.assert_card_identity(live, LIVE);
    game.assert_card_identity(payer, PAYER);

    let f1 = game.id(FILLER);
    let f2 = game.id(FILLER);
    assert_ne!(f1, f2, "the two fillers must be distinct instances");
    assert!(
        !game.db.is_live_card(f1),
        "precondition: the filler must not be a ライブ card"
    );
    assert!(
        game.db.is_live_card(live),
        "precondition: the target must be a ライブ card"
    );

    // Deck top = filler, filler, live.
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(f1);
    game.state.player1.main_deck.cards.push(f2);
    game.state.player1.main_deck.cards.push(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(f1);
    }
    game.give_energy(15);

    // Hand = ミア + the payer. Playing ミア removes her from hand,
    // so the optional cost「手札を1枚控え室に置いてもよい」offers
    // the payer as the only eligible card.
    game.add_to_hand(mia);
    game.add_to_hand(payer);
    game.play_to_stage(mia, MemberArea::LeftSide);

    // Pay the optional cost: select the payer (index 0 of the
    // remaining hand). Skipping it would abort the ability
    // (rules 9.7.3.1.1), so the effect never runs.
    assert!(
        game.has_pending_choice(),
        "the optional debut cost must be offered; got {:?}",
        game.pending_choice_summary()
    );
    game.select_indices(&[0]);

    // The effect runs with no further prompts: reveal until the
    // live card, bank the live card into hand, bank the two
    // fillers into the waitroom.
    game.drain_choices_strict(&[], &[]);

    // The live card is in hand.
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "the revealed live card must be added to 手札; hand is {:?}",
        game.state.player1.hand.cards
    );

    // Each filler is in the waitroom EXACTLY ONCE. The
    // `looked_at_remaining` step banks the pool in
    // `resolve_from_looked_at` and then `place_taken_cards`
    // re-places the same cards, so a double-add shows up here
    // as a duplicate.
    for &f in &[f1, f2] {
        let count = game
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .filter(|&&c| c == f)
            .count();
        assert_eq!(
            count, 1,
            "each filler must land in 控え室 exactly once; \
             card {} appears {} times in {:?}",
            f, count, game.state.player1.waitroom.cards
        );
    }

    // The payer (the paid cost) is in the waitroom too.
    assert!(
        game.state.player1.waitroom.cards.contains(&payer),
        "the paid cost card must be in 控え室; waitroom is {:?}",
        game.state.player1.waitroom.cards
    );

    // The `looked_at_remaining` placement recorded a movement
    // event: the fillers are in the turn's movement-event log.
    // Before the fix, `resolve_from_looked_at` banked the pool
    // with a bare `waitroom.add_card` that recorded nothing,
    // so an any-zone→waitroom watcher never saw the placement.
    let fillers_in_log = |f: i16| {
        game.state
            .turn_movements
            .iter()
            .any(|ev| ev.moved_card_id == f)
    };
    assert!(
        fillers_in_log(f1) && fillers_in_log(f2),
        "the looked_at_remaining discard must record a movement \
         event for each filler; turn_movements is {:?}",
        game.state
            .turn_movements
            .iter()
            .map(|ev| (ev.moved_card_id, format!("{:?}->{:?}", ev.source_zone, ev.dest_zone)))
            .collect::<Vec<_>>(),
    );
}
