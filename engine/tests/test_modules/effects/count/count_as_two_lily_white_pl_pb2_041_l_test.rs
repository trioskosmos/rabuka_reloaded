//! 春情ロマンティック `PL!-pb2-041-L` (常時 + ライブ開始時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   常時 — 自分の『lily white』のカードの効果によって、自分の成功ライブカード置き場に
//!   あるカードの枚数を数えるとき、このカードは2枚として扱う。
//!
//!   ライブ開始時 — 自分の成功ライブカード置き場に『lily white』のカードが2枚以上ある場合、
//!   このカードのスコアを＋１する。
//!
//! # The weight, observed through a real consumer
//!
//! The two abilities of this one card observe each other. The 常時 weight applies to
//! counts made by a **lily white** card's effect, and the ライブ開始時 IS a lily white
//! card counting the success live zone with a threshold of **2** — so a zone holding
//! nothing but this card is ONE card and misses the threshold, UNLESS the weight makes
//! it two. That observes the mechanic through printed behaviour rather than by querying
//! `count_as_multiple_weight` directly.
//!
//! # A consumer that does NOT work, recorded in the 039 file
//!
//! `PL!-pb2-039-L` has the same threshold and was briefly used as the observer here. It
//! has `unit: None`, so it is in no group and can never be a lily white counter. Its own
//! test file now pins that, so the mistake is not repeated.
//!
//! # A real engine bug this surfaced
//!
//! Driving the weight through the real phase flow put the same card id in BOTH the live
//! card zone and the success live card zone. `execute_modify_score` built its target pool
//! by chaining those two zones, so the card appeared twice and this card's own
//! 「このカードのスコアを＋１する」 applied TWICE: a single
//! activation produced +2. The pool is now order-preservingly deduplicated, and
//! `self_score_is_not_applied_once_per_zone_occurrence` pins it.

use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;

const ROMANTIC: &str = "PL!-pb2-041-L";
const FILLER: &str = "PL!-sd1-010-SD";

/// The weight, parsed: a `count_as_multiple` of 2, seen only by `lily white` effects.
#[test]
fn the_constant_parses_as_a_counting_weight_of_two() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let eff = {
        let owned = game
            .db
            .get_card(game.id(ROMANTIC))
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("常時"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| e.clone()))
            .expect("常時 ability has an effect")
    };
    assert_eq!(
        eff.action,
        rabuka_engine::ability::enums::ActionType::CountAsMultiple,
        "このカードは2枚として扱う — a counting weight, not a custom action"
    );
    assert_eq!(eff.count_as_any(), Some(2), "2枚として扱う");
    let by: Vec<String> = eff
        .counted_by_any()
        .map(|v| v.to_vec())
        .unwrap_or_default();
    assert_eq!(
        by,
        vec!["lily white".to_string()],
        "自分の『lily white』のカードの効果によって — the effects that SEE the weight. \
         NOT a filter on the counted card, so it must not sit in `group_names`."
    );
}

/// THE test. This card alone in the success zone, performing a LiveStart.
///
/// Unweighted, a zone holding nothing but this card is ONE `lily white` card against
/// the 2枚以上 threshold, so the score bonus must NOT apply. Weighted as two, it must.
///
/// This drives the real phase flow (set the live card, reach performance) rather than
/// hand-rolling a trigger, so it observes the same ライブ開始時 the live actually runs.
#[test]
fn this_card_alone_counts_as_two_and_earns_the_score_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let romantic = game.id(ROMANTIC);
    game.assert_card_identity(romantic, ROMANTIC);
    game.assert_card_score(romantic, 7);
    game.assert_card_in_group(
        romantic,
        "lily white",
        "the weighted card must itself be lily white",
    );

    // The card is already in the success zone AND is the live being performed. The 常時
    // rule is scoped to the success live zone, so this is the only placement where the
    // weight is live at all.
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(romantic);

    let filler = game.id(FILLER);
    game.add_to_hand(romantic);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    fill_decks(&mut game, filler);

    let mut h = BaseHeart {
        hearts: HeartMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 30);
    game.state.player1.stage_hearts = Some(h);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(romantic);
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    assert_eq!(
        game.state.mods.get_score_modifier(romantic),
        1,
        "このカードは2枚として扱う — alone in the 成功ライブカード置き場 this card is \
         ONE 『体の』 card against a 2枚以上 threshold, so the 常時 weight is the \
         only thing that can satisfy it, and the +1 must follow"
    );
}

/// A card id must score once per activation, not once per zone it is visible in.
///
/// The live being performed is the live card AND an entry in the success live card zone
/// once it has succeeded, so the score target pool saw the same id twice. This is the
/// same card and the same activation, so 「このカードのスコアを＋１する」 means +1.
#[test]
fn self_score_is_not_applied_once_per_zone_occurrence() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let romantic = game.id(ROMANTIC);
    game.assert_card_identity(romantic, ROMANTIC);

    // A plain qualifying setup, then the SAME id also left in the success zone.
    let lw1 = game.id("PL!-pb1-029-SRL"); // lily white, score 1
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(lw1);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(romantic);

    let filler = game.id(FILLER);
    game.add_to_hand(romantic);
    game.add_to_hand(filler);
    game.add_to_hand(filler);
    fill_decks(&mut game, filler);

    game.advance_to_phase(Phase::LiveCardSetFirstAttacker);
    game.set_live_card(romantic);
    game.advance_to_phase(Phase::FirstAttackerPerformance);

    // lw1 + this card, which the 常時 weight counts as two → the count is 3, well over the
    // 2枚以上 threshold, so the effect fires. It must still be worth exactly +1.
    assert_eq!(
        game.state.mods.get_score_modifier(romantic),
        1,
        "このカードのスコアを＋１する is a single +1 on a single card — the \
         target pool must not score the card once per zone it appears in, which made \
         one activation pay out +2"
    );
}
