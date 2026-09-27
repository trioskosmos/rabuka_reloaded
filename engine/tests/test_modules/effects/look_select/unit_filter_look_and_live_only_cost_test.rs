//! 米女メイ `PL!SP-bp7-018-N` — a `look_and_select` whose cost is TYPE-GATED, and
//! whose whole remainder hangs off paying it.
//!
//! 「**手札のライブカードを**1枚控え室に置いてもよい：自分のデッキの上からカードを5枚見る。
//! その中から1枚手札に加え、残りを控え室に置く。」
//!
//! The measured prompt sequence on the paid branch is:
//!
//! ```text
//!   prompt #1  SelectCard zone=hand      count=1 allow_skip=true  card_type=Some("live_card")
//!   prompt #2  SelectCard zone=looked_at count=1 allow_skip=false options=Some(5)
//!   FINAL      deck=35 hand=1 waitroom=5
//! ```
//!
//! Three things fall out of that, all of which this file pins:
//!
//!   1. THE COST IS TYPE-GATED — the prompt itself carries
//!      `card_type=Some("live_card")`. That is the engine enforcing
//!      「手札の**ライブカード**を1枚」, and it is why a members-only hand cannot pay
//!      it however many members it holds.
//!   2. THE RETRIEVAL IS NOT SKIPPABLE — `allow_skip=false` with five options. The
//!      「もよい」 in the printed text attaches to the COST, not to taking a card.
//!   3. THE ARITHMETIC IS 5 / 1 / 4, plus the cost card in the waitroom: five leave
//!      the deck, one reaches the hand, five end in the waitroom.
//!
//! ## The declined branch is the colon-gate, and it is worth a test
//!
//! With a members-only hand the cost is unpayable, the engine treats that as
//! DECLINED, and its colon-gate convention then prevents the gated effect from
//! firing: no second prompt, nothing off the deck, nothing banked. That is
//! `ability/cost/handlers.rs:364` — "this cost colon-gates the effect … prevents the
//! gated effect from firing" — observed rather than inferred. It is also the easiest
//! thing in this family to mistake for a broken ability, so it is asserted.
//!
//! ## An engine defect found here, and left unfixed
//!
//! 平安名すみれ `PL!SP-pb1-015-N` has the same shape with a UNIT-level group filter
//! (『CatChu!』), and her printed retrieval is UNREACHABLE. Measured, with the 『CatChu!』
//! member sitting third in the looked five:
//!
//! ```text
//!   card_matches_group_str(CatChu!) = true    <- the matcher accepts the card
//!   prompt #1  zone=hand count=1 allow_skip=true
//!     after [0]:  deck=80 hand=0 waitroom=6 looked=[]
//!   FINAL       deck=80 hand=0 waitroom=6       <- all five banked, no selection prompt
//! ```
//!
//! Five cards leave the deck and ALL SIX (the cost card plus the five) reach the
//! waitroom, and the `looked_at` selection prompt never appears. So
//! `looked_at_matching_indices` (ability/look.rs:52) finds ZERO matches even though
//! `card_matches_group_str` accepts the same card.
//!
//! Where a fix belongs, and what is already ruled out. The group is NOT lost in the
//! data and is NOT mis-decoded:
//!
//! - the parsed `select_action` carries `group_names: ["CatChu!"]` — present and
//!   correct in abilities.json;
//! - `effect_decoder_gen.rs` reads `group_names` and `card_names` into SEPARATE
//!   fields (`ek.group_names` / `ek.card_names`), and `build_filter` carries both, so
//!   there is no cross-wiring at decode;
//! - `CardFilter::from_effect` maps `group_names` into BOTH `group` and `groups`
//!   (util.rs:1539-1543), and `Filter::check_group` (util.rs:1220) consults them;
//! - `card_matches_group_str(db, <that same card>, Some("CatChu!"))` returns TRUE,
//!   measured.
//!
//! So the group arrives, the matcher accepts the card, and
//! `looked_at_matching_indices` (look.rs:45-54, via `CardFilter::from_effect`) still
//! yields ZERO indices. Whatever else the built filter carries is rejecting every
//! looked card — the next thing to inspect is what `from_effect` populates for this
//! effect shape beyond the group (util.rs:1526-1565), particularly
//! `name_fragments`, which is the field most likely to exclude a card on its NAME.
//!
//! メイ's identical shape with NO filter retrieves reliably, which is the control:
//! the look, the count, the cost and the banking all work, and only the filtered
//! selection is broken. Left unfixed here because `from_effect` is shared by every
//! filtered selection in the game, and guessing at it is not a change worth making
//! without the field in hand.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MEI: &str = "PL!SP-bp7-018-N";
const FILLER: &str = "PL!-sd1-010-SD";
const LIVE: &str = "PL!-sd1-019-SD";
const LOOK_COUNT: usize = 5;

fn fill(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.live_card_zone.cards.clear();
    game.give_energy(20);
}

/// Stack `LOOK_COUNT` cards as the FIRST cards the look sees, then bulk behind them.
///
/// Order is load-bearing: `MainDeck::draw()` removes index 0, so seeding first and
/// filling after puts them on top. The other way round leaves the look drawing
/// fillers, which looks right and measures nothing.
fn seeded_top(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..LOOK_COUNT {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

/// Play from hand and answer every prompt with its first option.
fn play_and_answer(game: &mut TestGame, card: i16) {
    game.play_to_stage(card, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 8 {
        guard += 1;
        game.select_indices(&[0]);
    }
}

/// The paid branch: the cost is paid, five are looked at, one is taken, four banked.
///
/// `waitroom = 1 + 4` is the load-bearing number — it proves the cost card AND the
/// four-card remainder both arrived, which neither "a card was taken" nor a deck
/// count can distinguish from the cost having been skipped.
#[test]
fn mei_paid_cost_looks_five_takes_one_and_banks_the_rest() {
    let mut game = TestGame::new(load_real_database());
    let mei = game.id(MEI);
    let live = game.id(LIVE);
    game.assert_card_identity(mei, MEI);
    assert_eq!(game.db.get_card(mei).unwrap().name, "米女メイ");

    fill(&mut game);
    game.add_to_hand(mei);
    game.state.player1.hand.cards.push(live);
    seeded_top(&mut game);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    play_and_answer(&mut game, mei);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "three moves on the hand: the played card leaves it, the cost takes the live \
         card, the retrieval puts one back. The net is -1, and -1 rather than -2 is \
         what distinguishes a paid cost with a retrieval from a paid cost without one."
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 1 + (LOOK_COUNT - 1),
        "『残りを控え室に置く』 — the cost card plus the four unbanked-looked cards"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - LOOK_COUNT,
        "exactly LOOK_COUNT cards left the deck for the look"
    );
    assert_eq!(
        game.state.looked_at_cards.len(),
        0,
        "and the looked-at pool is empty once the selection resolves"
    );
}

/// THE DECLINED BRANCH, which is the colon-gate convention working.
///
/// A hand of members cannot pay 「手札のライブカードを1枚控え室に置いてもよい」 — the
/// prompt is gated on a ライブカード, so three members do not help. Unpayable is
/// treated as declined, and the colon-gate then prevents the gated effect from firing:
/// no second prompt, nothing off the deck, nothing banked.
///
/// Asserted because it is easy to misread as a broken ability, and because it is the
/// branch a fixture hits by accident — this file's own first draft did.
#[test]
fn mei_declined_live_only_cost_cancels_the_look_by_the_colon_gate() {
    let mut game = TestGame::new(load_real_database());
    let mei = game.id(MEI);
    fill(&mut game);
    // Members only, and asserted to be members: the cost names a ライブカード, so
    // nothing here can pay it whatever the hand size.
    for _ in 0..3 {
        let id = game.new_id(FILLER);
        assert!(
            !game
                .db
                .get_card(id)
                .map(|c| c.card_type == rabuka_engine::card::CardType::Live)
                .unwrap_or(true),
            "precondition: these must NOT be live cards, or the type-gated cost is \
             payable and the colon-gate is never reached"
        );
        game.state.player1.hand.cards.push(id);
    }
    game.add_to_hand(mei);
    seeded_top(&mut game);
    let deck_before = game.state.player1.main_deck.cards.len();
    let waitroom_before = game.state.player1.waitroom.cards.len();

    play_and_answer(&mut game, mei);

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "「…置いてよい：自分のデッキの上からカードを5枚見る」 is gated by the cost, and \
         an unpayable cost is a decline, so the look never runs"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before,
        "and nothing is banked: with the look cancelled there is no remainder either"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "the hand is the three members left after メイ was played — no card discarded, \
         none retrieved. 3 rather than 2 is what says the cost was not paid either."
    );
}
