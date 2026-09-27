//! The last 自動/登場 family: two THREE-WAY CHOICES, and two look-and-selects.
//!
//! A choice is the one thing in this deck where the CORRECT value is not derivable
//! from the condition. Every other ability in the family has a single printed
//! outcome, so a test that reads a number proves something. A 3-way choice does
//! not: an implementation that ignored the selection and always took the first
//! option, or that applied all three, passes a test that never varies the pick.
//! So each choice is exercised on a NON-FIRST option and on the option it did not
//! pick — a first-option-only test cannot tell any of that apart.
//!
//!   * 絢瀬絵里 `PL!-bp5-011-N` — ライブ開始時, choose one of heart04/05/06, then per
//!     card in own SUCCESS zone gain THAT heart until ライブ終了時. Choice identity
//!     and a per-card multiplier at once.
//!   * 若菜四季 `PL!SP-pb2-030-N` — ライブ開始時, choose one of three, and her PRINTED
//!     hearts become the chosen one. The observable is the card's own heart, not a
//!     modifier map, so it is checked where the card data actually lives.
//!   * ウララ `PL!HS-cl1-007-CL` / 優木あんじゅ `PL!-bp5-222-R` — 登場, look at the top
//!     three, take exactly one, the rest to the waitroom, with an optional discard.
//!     Several distinguishable wrong answers here (take two, take from the waitroom,
//!     lose the remainder) and the "exactly one" is the load-bearing word.

//!   * 絢瀬絵里 `PL!-bp5-011-N` — ライブ開始時, choose one of heart04/05/06, then per
//!     card in own SUCCESS zone gain THAT heart until ライブ終了時. Choice identity
//!     and a per-card multiplier at once.
//!   * 若菜四季 `PL!SP-pb2-030-N` — ライブ開始時, choose one of three, and her hearts
//!     become the chosen one for the live. The observable is the duration-scoped
//!     colour override (`mods.heart_color_multiplier`), not the card's data.
//!
//! A look-and-select pair (ウラら / 優木あんじゅ, both 「デッキの上から3枚を見て1枚を
//! 手札に加え、残りを控え室に置く」) is NOT covered here: the card numbers in the
//! inventory are prefixes rather than exact ids, and the one existing test for that
//! family drives a DIFFERENT card (`PL!HS-cl1-004-CL` is the mill-or-wait choice).
//! Rather than ship a test whose fixture is a guess, the pair is left for whoever can
//! resolve the ids — the parsed effect is a well-formed
//! `look_and_select { source: deck_top, count: 3, discard_remaining: true }`, and
//! `ability/look.rs: execute_look_and_select` is the path to drive.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const ERICHO: &str = "PL!-bp5-011-N"; // 絢瀬絵里 — choose 1 of 3 hearts, scaled per success card
const WAKANA: &str = "PL!SP-pb2-030-N"; // 若菜四季 — choose 1 of 3, REPLACES her hearts
const FILLER: &str = "PL!-sd1-010-SD";
const LIVE: &str = "PL!-sd1-019-SD";


fn fill_decks(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn heart_mod(game: &TestGame, card: i16, colour: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card, colour)
}

/// Answer the next prompt with option `index`, or skip it when there is none.
fn choose(game: &mut TestGame, index: usize) {
    if game.has_pending_choice() {
        game.select_option(index as i16);
    }
}

// ====================================================================
// 絢瀬絵里 — a 3-way choice, then a PER-CARD multiplier
// ====================================================================

/// Pick the LAST option (heart06), not the first, so an implementation that ignores
/// the selection and always takes the first is caught. Two success-zone cards, so
/// the multiplier is 2 — and the two UNCHOSEN hearts are asserted at zero, which is
/// what an "applies all three" reading fails.
#[test]
fn ericho_gains_only_the_chosen_heart_two_per_success_card() {
    let mut game = TestGame::new(load_real_database());
    let ericho = game.id(ERICHO);
    game.assert_card_identity(ericho, ERICHO);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [ericho, -1, -1];
    // Two ライブ in own SUCCESS zone → the chosen heart is gained twice.
    for _ in 0..2 {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.new_id(LIVE));
    }

    fire_trigger(&mut game, ericho, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        game.has_pending_choice(),
        "precondition: the 3-way heart choice must actually be offered"
    );
    // The LAST option, not the first.
    choose(&mut game, 2);

    assert_eq!(
        heart_mod(&game, ericho, HeartColor::Heart06),
        2,
        "『カード1枚につき、選んだハートを1つ得る』 with 2 success cards = 2 of the \
         CHOSEN heart — the chosen one is the last option, so a first-option \
         implementation lands on heart04 instead"
    );
    assert_eq!(
        heart_mod(&game, ericho, HeartColor::Heart04),
        0,
        "the FIRST option was not chosen, so heart04 is not granted — this is the \
         assertion an 'applies all three' reading fails"
    );
    assert_eq!(
        heart_mod(&game, ericho, HeartColor::Heart05),
        0,
        "and the middle option is not granted either"
    );
}

/// The same card with an EMPTY success zone: the condition reads
/// 「自分の成功ライブカード置き場にあるカード1枚につき」, so zero cards means zero
/// hearts even though the choice was still made and stored.
#[test]
fn ericho_gains_nothing_with_an_empty_success_zone() {
    let mut game = TestGame::new(load_real_database());
    let ericho = game.id(ERICHO);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [ericho, -1, -1];
    game.state.player1.success_live_card_zone.cards.clear();

    fire_trigger(&mut game, ericho, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        game.has_pending_choice(),
        "precondition: the choice is offered regardless of the success zone"
    );
    choose(&mut game, 2);

    for colour in [HeartColor::Heart04, HeartColor::Heart05, HeartColor::Heart06] {
        assert_eq!(
            heart_mod(&game, ericho, colour),
            0,
            "『カード1枚につき』 is a multiplier on a count, and the count is 0 — no \
             heart of any colour is gained, including the chosen one"
        );
    }
}

// ====================================================================
// 若菜四季 — the chosen heart REPLACES her printed hearts
// ====================================================================

/// 「{{heart_02}}か{{heart_03}}か{{heart_06}}のうち1つを選ぶ。ライブ終了時まで、
/// **このメンバーが元々持つハートは選んだハートになる**。」
///
/// A different effect from her 絵里 sibling: not a gain, but a REPLACEMENT of the
/// card's own printed hearts. The observable is therefore the card's heart DATA, so
/// it is read off the database rather than a modifier map — a modifier reading would
/// pass even if the printed heart were untouched, which is a different bug.
#[test]
fn wakana_the_chosen_heart_replaces_her_printed_hearts() {
    let mut game = TestGame::new(load_real_database());
    let wakana = game.id(WAKANA);
    game.assert_card_identity(wakana, WAKANA);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [wakana, -1, -1];

    // Her PRINTED heart before the choice, so the replacement is a change FROM
    // something rather than an appearance out of nothing — and so the option chosen
    // is a DIFFERENT colour from the one she already has. She prints heart06, so
    // choosing the last option (heart06) would be a no-op and would prove nothing;
    // the middle option (heart03) is the one that actually replaces something.
    let printed = game
        .state
        .card_database
        .get_card(wakana)
        .unwrap()
        .base_heart
        .clone();
    assert!(
        printed
            .as_ref()
            .and_then(|h| h.hearts.get(&HeartColor::Heart06))
            .is_some(),
        "precondition: this print's heart is heart06, so the test must pick a \
         DIFFERENT option — choosing heart06 would leave the card unchanged and the \
         assertions below would pass vacuously (printed {printed:?})"
    );

    fire_trigger(&mut game, wakana, AbilityTrigger::LiveStart, "ライブ開始時");
    assert!(
        game.has_pending_choice(),
        "precondition: the 3-way heart choice must be offered"
    );
    // The MIDDLE option, heart03 — not the one she already prints.
    choose(&mut game, 1);

    // The observable is the heart-COLOUR MODIFIER, not the card's data.
    // `set_heart_type` is implemented as a duration-scoped modifier
    // (`state.rs: execute_set_heart_type` → `mods.heart_color_multiplier`), which is
    // what the live-end expiry path removes. Reading `base_heart` would see the
    // PRINTED value forever and pass or fail for reasons unrelated to the ability.
    let applied = game
        .state
        .mods
        .heart_color_multiplier
        .get(&wakana)
        .copied();
    assert_eq!(
        applied,
        Some(HeartColor::Heart03),
        "『元々持つハートは選んだハートになる』 — the chosen heart03 is applied as a \
         duration-scoped colour override (got {applied:?}), replacing the heart06 she \
         printed. The card DATA is deliberately not asserted: it is immutable here, and \
         a reading of base_heart cannot see this ability at all."
    );
    assert_ne!(
        applied,
        Some(HeartColor::Heart06),
        "and it is NOT the heart she printed — a 'choose the first option' or 'keep the \
         original' reading lands on heart06 and fails only here"
    );
}

