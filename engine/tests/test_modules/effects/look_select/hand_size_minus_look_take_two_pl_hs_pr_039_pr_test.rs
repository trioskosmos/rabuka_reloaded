//! 百生吟子 `PL!HS-PR-039-PR` and 米女メイ `PL!SP-PR-028-PR` (登場) — one shared ability.
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場 — 自分のデッキの上から、10から自分の手札の枚数を引いた数に等しい枚数のカードを見る。
//!   その中から2枚まで手札に加える。残りを控え室に置く。
//!
//! In English: on debut, look at (10 − my hand size) cards from the top of my deck; take
//! up to 2 of them into hand; the rest go to the 控え室.
//!
//! # Why this file exists
//!
//! The look count is a `dynamic_count` whose reference arrived from the parser as raw
//! prose — 「自分のデッキの上から、10から自分の手札の枚数を引いた数」 — which the engine's
//! dispatcher did not recognize and resolved to **0**, so the ability silently looked at
//! nothing and every downstream assertion would have measured an effect that never ran.
//! It was the only unresolved reference in the whole corpus (`abilities.json`, 47
//! dynamic_count nodes).
//!
//! The fix is end-to-end, and this file pins every link of it:
//!
//! 1. `cards/ability_extraction/parser.py::_canonicalize_dynamic_counts` rewrites the
//!    prose to `hand_size_below_base` + `base_reference="10"`.
//! 2. `engine/src/ability/dynamic_count.rs` resolves that token to
//!    `10.saturating_sub(hand_size)` — saturating, because a hand of 10 or more is legal
//!    and must look at nothing rather than underflow.
//! 3. The tests below assert the printed look count, the 2-card take cap, and the
//!    remainder routing, for both prints.
//!
//! # Setup idiom
//!
//! The look's hand size is read at the moment the card enters the stage, so the hand is
//! assembled BEFORE the play: the played card itself leaves the hand, and the debut
//! trigger then sees exactly `hand_extra` cards. Setup lifted from
//! `effects/look_select/pb2_debut_look_four_take_mus_live_need_hearts_eight_pl_pb2_008_pp_test.rs`.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// 百生吟子 — the print whose hand this file is named after.
const HYOUGI: &str = "PL!HS-PR-039-PR";
/// 米女メイ — the second print carrying the identical ability (ab#0).
const MEI: &str = "PL!SP-PR-028-PR";
/// 高坂穂乃果 — zero-ability filler used for deck bulk and hand padding.
const FILLER: &str = "PL!-sd1-010-SD";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pools {
    deck: usize,
    hand: usize,
    waitroom: usize,
}

fn pools(game: &TestGame) -> Pools {
    Pools {
        deck: game.state.player1.main_deck.cards.len(),
        hand: game.state.player1.hand.cards.len(),
        waitroom: game.state.player1.waitroom.cards.len(),
    }
}

struct Run {
    before: Pools,
    offer: Pools,
    after: Pools,
    offered: usize,
}

/// Stand `card_no` with `hand_extra` fillers beside it, play it, and read the look's
/// offer **before answering it**. Returns (pools before the play, pools at the offer,
/// the game, how many cards the look turned over).
fn setup(card_no: &str, hand_extra: usize) -> (Pools, Pools, TestGame, usize) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(card_no);
    game.assert_card_identity(me, card_no);

    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.add_to_hand(me);
    for _ in 0..hand_extra {
        game.state.player1.hand.cards.push(game.new_id(FILLER));
    }
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];

    let before = pools(&game);
    game.play_to_stage(me, MemberArea::Center);

    // 登場 has no cost, so the first prompt IS the look. Read it before answering —
    // `pending_choice_option_count()` is the candidate count (`filtered_indices.len()`),
    // i.e. how many cards the look actually turned over.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "the debut look must be the first prompt; there is no cost gate on this ability"
    );
    let offered = game.pending_choice_option_count().unwrap_or(0);
    let offer = pools(&game);
    (before, offer, game, offered)
}

/// Drive one look to completion, answering with `take` cards (`0` declines the
/// skip-able selection) and skipping the follow-up prompts.
fn run(card_no: &str, hand_extra: usize, take: usize) -> Run {
    let (before, offer, mut game, offered) = setup(card_no, hand_extra);

    if offered > 0 {
        let picks: Vec<usize> = (0..take).collect();
        game.select_indices(&picks);
        // Every partial answer is followed by a 「残りの見たカード…最大N枚まで選択
        // （スキップ可）」 re-prompt. Answering IT too would keep taking cards, so a
        // player who wants exactly `take` skips here — the skip is what ends the
        // selection and banks the remainder.
        if take > 0 && game.has_pending_choice() {
            game.select_indices(&[]);
        }
    }
    assert!(
        !game.has_pending_choice(),
        "the answer + skip above settles the look; a prompt that survives it is {:?}",
        game.pending_choice_type()
    );
    let after = pools(&game);

    Run {
        before,
        offer,
        after,
        offered,
    }
}

/// THE test: the look count is 10 − hand size, measured on the prompt and on the deck.
///
/// 「10から自分の手札の枚数を引いた数に等しい枚数のカードを見る」 — the count is read at
/// the moment of the debut, when the hand holds exactly `hand_extra` cards (the played
/// card has already left it). With 3 in hand the look must turn over 7.
#[test]
fn the_look_count_is_ten_minus_the_current_hand_size() {
    let run = run(HYOUGI, 3, 1);
    assert_eq!(
        run.offered, 7,
        "10 − 3 手札 = 7枚を見る — the prose dynamic_count used to resolve to 0, which is \
         exactly what this assertion forbids"
    );
    assert_eq!(
        run.before.deck - run.offer.deck,
        7,
        "the looked cards leave the deck: deck 40 → 33 (before {:?}, offer {:?})",
        run.before,
        run.offer
    );
}

/// A bigger hand looks at fewer cards — the count tracks the hand, it is not a constant.
#[test]
fn a_larger_hand_shrinks_the_look() {
    let run = run(HYOUGI, 6, 1);
    assert_eq!(
        run.offered, 4,
        "10 − 6 手札 = 4枚 — a constant 7 (or the old 0) would fail here"
    );
    assert_eq!(
        run.before.deck - run.offer.deck,
        4,
        "the deck pays exactly the looked count (before {:?}, offer {:?})",
        run.before,
        run.offer
    );
}

/// 「その中から2枚まで手札に加える」 — the take is capped at 2, and everything not taken
/// is banked in the waitroom.
#[test]
fn up_to_two_go_to_hand_and_the_rest_are_banked() {
    let hand_extra = 0;
    let run = run(HYOUGI, hand_extra, 2);
    let looked = 10 - hand_extra;
    assert_eq!(
        run.offered, looked,
        "an empty hand looks at the full 10 (hand held only the played card itself)"
    );
    assert_eq!(
        run.after.hand,
        hand_extra + 2,
        "2枚まで手札に加える — two cards were taken"
    );
    assert_eq!(
        run.after.waitroom,
        looked - 2,
        "残りを控え室に置く — the other 8 looked cards are banked"
    );
    assert_eq!(
        run.offer.deck,
        run.after.deck,
        "nothing further leaves the deck after the look"
    );
    assert_eq!(
        run.after.hand + run.after.waitroom,
        // The played card left the hand for the stage, so the hand started at
        // `hand_extra` the moment the debut fired.
        run.before.hand - 1 + looked,
        "conservation: every card that left the deck or was already in hand is still \
         accounted for (before {:?}, offer {:?}, after {:?})",
        run.before,
        run.offer,
        run.after
    );
}

/// The cap holds even when the player keeps answering the re-prompts.
///
/// 「2枚まで」 bounds the TOTAL. Each answer used to be measured on its own, so a
/// player who picked one card at a time was offered "up to 1 more" forever and
/// banked the whole look — the loop below is exactly that greedy drive, and with the
/// fix it must stop at 2.
#[test]
fn one_at_a_time_answers_cannot_walk_past_the_cap() {
    let (before, offer, mut game, offered) = setup(HYOUGI, 0);
    assert_eq!(offered, 10, "an empty hand looks at the full 10");

    let mut answers = 0;
    game.select_indices(&[0]);
    answers += 1;
    // Loop on the TYPE, not on pendency: the greedy player keeps answering for as
    // long as the 「残りの見たカード…最大N枚まで選択」 prompt comes back. The bound
    // only stops a hypothetical infinite loop — `assert_eq!(answers, 2)` below is
    // the real verdict.
    while game.pending_choice_type().as_deref() == Some("SelectCard") {
        game.select_indices(&[0]);
        answers += 1;
        assert!(
            answers <= 20,
            "the 「2枚まで」 cap never closed the re-prompt (before {before:?}, offer \
             {offer:?}, after {:?})",
            pools(&game)
        );
    }
    let after = pools(&game);

    assert_eq!(
        answers, 2,
        "2枚まで手札に加える — after two one-card answers the look is finished, so no \
         further 「残り…最大1枚まで選択」 prompt may appear (before {before:?}, offer \
         {offer:?}, after {after:?})"
    );
    assert_eq!(
        after.hand, 2,
        "exactly two cards reached the hand"
    );
    assert_eq!(
        after.waitroom, 8,
        "残りを控え室に置く — the other eight looked cards are banked"
    );
    assert_eq!(after.deck, before.deck - 10, "the look left the deck once");
}

/// Taking fewer than the cap is legal, and the remainder still goes to the waitroom.
#[test]
fn taking_one_leaves_the_remainder_in_the_waitroom() {
    let hand_extra = 3;
    let looked = 10 - hand_extra;
    let run = run(HYOUGI, hand_extra, 1);
    assert_eq!(run.offered, looked);
    assert_eq!(
        run.after.hand,
        hand_extra + 1,
        "one of the looked cards joined the hand"
    );
    assert_eq!(
        run.after.waitroom,
        looked - 1,
        "残りを控え室に置く — the six unselected looked cards are banked"
    );
}

/// Declining the skip-able selection banks EVERYTHING the look turned over.
#[test]
fn declining_the_take_banks_the_whole_look() {
    let hand_extra = 4;
    let looked = 10 - hand_extra;
    let run = run(HYOUGI, hand_extra, 0);
    assert_eq!(run.offered, looked, "the look still runs when nothing is taken");
    assert_eq!(
        run.after.hand, hand_extra,
        "not taking anything leaves the hand where it was"
    );
    assert_eq!(
        run.after.waitroom,
        looked,
        "残りを控え室に置く — with no take, the whole look is banked"
    );
}

/// A hand of 10 or more means 10 − hand ≤ 0: look at nothing, touch nothing.
///
/// This is the saturation half of the fix. An unsaturating subtraction would underflow
/// (and in release wrap to a huge count); a look of 0 must simply be a no-op.
#[test]
fn a_hand_of_ten_or_more_looks_at_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(HYOUGI);
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id(FILLER));
    }
    game.add_to_hand(me);
    for _ in 0..10 {
        game.state.player1.hand.cards.push(game.new_id(FILLER));
    }
    game.give_energy(20);
    game.state.player1.stage.stage = [-1, -1, -1];
    let before = pools(&game);

    game.play_to_stage(me, MemberArea::Center);
    assert!(
        !game.has_pending_choice(),
        "a look of 0 cards must not open a prompt at all (opened {:?})",
        game.pending_choice_type()
    );
    let after = pools(&game);

    assert_eq!(
        after.deck,
        before.deck,
        "10から自分の手札の枚数を引いた数 ≤ 0 means no cards are looked at (before \
         {before:?}, after {after:?})"
    );
    assert_eq!(
        after.waitroom,
        before.waitroom,
        "nothing is banked either — with 10 in hand the look turns over zero cards"
    );
    assert_eq!(
        after.hand,
        before.hand - 1,
        "the hand only lost the card that was played (before {before:?}, after {after:?})"
    );
}

/// The second print carries the same ability, so it behaves identically.
#[test]
fn the_second_print_behaves_the_same() {
    let run = run(MEI, 3, 1);
    assert_eq!(
        run.offered, 7,
        "米女メイ shares ab#0 with 百生吟子 — 10 − 3 手札 = 7枚を見る"
    );
    assert_eq!(run.after.hand, 3 + 1, "and takes the same way");
    assert_eq!(run.after.waitroom, 6, "with the same banking");
}

/// Pin the parse so a re-extraction cannot silently regress the dispatch token.
#[test]
fn the_parsed_effect_carries_the_dispatchable_reference() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card_no in [HYOUGI, MEI] {
        let id = game.id(card_no);
        let eff = {
            let owned = game.db.get_card(id).unwrap().resolved_abilities().find(|a| {
                a.triggers.as_deref() == Some("登場")
            });
            owned
                .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
                .expect("登場 ability has an effect")
        };
        assert_eq!(
            eff.action,
            rabuka_engine::ability::enums::ActionType::LookAndSelect,
            "{card_no}: その中から…手札に加える。残りを控え室に置く is a look-and-select"
        );
        // The look half must be a LOOK, not a draw. It used to parse as
        // `draw_card → hand` because action_012_draw_card matched the arithmetic
        // 「引いた数」 (引く as subtract) 30 rules ahead of action_042_look_at, so the
        // looked cards were drawn straight into hand and the select never ran.
        let look = eff
            .look_action()
            .map(|b| b.as_ref())
            .unwrap_or_else(|| panic!("{card_no}: the look half must exist"));
        assert_eq!(
            look.action,
            rabuka_engine::ability::enums::ActionType::LookAt,
            "{card_no}: 見る is a look_at — a draw_card here bypasses the look-and-select \
             entirely and hands the player the whole look"
        );
        assert_eq!(
            look.source.map(|z| z.as_str()),
            Some("deck_top"),
            "{card_no}: 自分のデッキの上から"
        );
        let dc = look
            .dynamic_count_any()
            .expect("the look count is dynamic — 10から自分の手札の枚数を引いた数");
        assert_eq!(
            dc.reference.as_deref(),
            Some("hand_size_below_base"),
            "{card_no}: the prose reference must be the engine-dispatchable token; raw \
             Japanese resolves to 0 and the ability silently does nothing"
        );
        assert_eq!(
            dc.base_reference.as_deref(),
            Some("10"),
            "{card_no}: 10から… — the subtrahend end of the clause lives in base_reference"
        );
        assert_eq!(
            dc.mode.as_deref(),
            Some("equals"),
            "{card_no}: 等しい枚数 — the count IS 10 − hand, not a bound on it"
        );
    }
}
