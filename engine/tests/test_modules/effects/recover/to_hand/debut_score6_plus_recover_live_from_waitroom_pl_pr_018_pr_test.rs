//! 東條 希 `PL!-PR-018-PR` 登場 — recover a live card of printed score 6+ from the waitroom.
//!
//! > {{toujyou.png|登場}}自分の控え室から、**スコア6以上の**ライブカードを1枚手札に加える。
//!
//! ## Why this clause is easy to lose
//!
//! 「スコア6以上の」 is a filter on the printed SCORE of the retrieved card, and until the
//! fix recorded in docs/ABILITY_AUDIT_2026-09-30.md §A2 the parse carried **no filter at
//! all** — the effect was a bare `move_cards` from the discard, so *any* live card in the
//! waitroom qualified. Nothing in the tree exercised this card, so the loss was invisible.
//!
//! There is no score filter in the engine either: `CardFilter` has no score dimension. The
//! clause rides on `cost_limit`, because `card_matches_cost_limit_op`
//! (`engine/src/ability/util.rs`) resolves the compared value as `c.cost.or(c.score)` and a
//! live card has no cost — so for a live card `cost_limit` *is* the score. That is why the
//! parse carries `cost_limit: 6, cost_limit_operator: ">="` rather than a bespoke field.
//!
//! ## Fixtures sit either side of the bound
//!
//! | card | score | role |
//! |---|---|---|
//! | `PL!-bp3-021-L` 愛してるばんざーい! | **6** | the exact bound — qualifies |
//! | `PL!S-PR-024-PR` 勇気はどこに?君の胸に! | **5** | one below — must NOT qualify |
//! | `PL!S-pb2-019-L` WATER BLUE NEW WORLD | **7** | above the bound |
//!
//! All three print no abilities, so nothing else can move a card during the 登場.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const NOZOMI: &str = "PL!-PR-018-PR";
/// score 6 — the exact bound the card prints.
const SCORE_6: &str = "PL!-bp3-021-L";
/// score 5 — one below.
const SCORE_5: &str = "PL!S-PR-024-PR";
/// score 7 — above.
const SCORE_7: &str = "PL!N-bp5-029-L";

/// Play NOZOMI with `pool` seeded into the waitroom plus two extra score-5 lives, and
/// report the card numbers that reached hand.
fn debut_with_waitroom(pool: &[&str]) -> (Vec<String>, TestGame) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    // Pin every fixture's printed score BEFORE staging, so a substituted printing cannot
    // move a card to the other side of the bound and quietly invert the test.
    for no in pool {
        let id = game.id(no);
        game.assert_card_identity(id, no);
        let score = game
            .db
            .get_card(id)
            .map(|c| c.score)
            .unwrap_or_else(|| panic!("{no} is not in the database"))
            .unwrap_or_else(|| panic!("{no} has no printed score, so it cannot pin the bound"));
        assert!(
            (5..=7).contains(&score),
            "precondition: {no} must print a score in the range this test discriminates, got {score}"
        );
        game.assert_card_score(id, i32::from(score));
        game.state.player1.waitroom.cards.push(id);
    }
    // TWO non-qualifying lives, so the zone is never a single card and the engine cannot
    // resolve by "there is only one thing to take".
    game.state.player1.waitroom.cards.push(game.new_id(SCORE_5));
    game.state.player1.waitroom.cards.push(game.new_id(SCORE_5));

    let nozomi = game.id(NOZOMI);
    game.assert_card_identity(nozomi, NOZOMI);
    game.assert_card_cost(nozomi, 15);
    game.add_to_hand(nozomi);
    game.give_energy(20);
    game.play_to_stage(nozomi, MemberArea::Center);
    game.drain_auto_ability_choices();

    let names = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter_map(|&c| game.db.get_card(c).map(|k| k.card_no.to_string()))
        .collect();
    (names, game)
}

/// The printed bound itself: exactly 6 qualifies and is the card taken.
#[test]
fn a_live_of_score_six_is_the_exact_bound_and_is_taken() {
    let (hand, _game) = debut_with_waitroom(&[SCORE_6]);
    assert!(
        hand.iter().any(|c| c == SCORE_6),
        "スコアの合計が… no — this is スコア6以上: a live printing exactly 6 satisfies it, so \
         {SCORE_6} must reach hand. Got hand card numbers {hand:?}"
    );
}

/// A score-5 live must NOT be offered, even alone in the zone.
///
/// The waitroom also holds a second score-5 copy (see the helper), so the zone is not empty
/// and the ability has real work to decline. Without the printed filter every live in the
/// zone qualifies and one of these would be taken.
#[test]
fn a_live_of_score_five_is_below_the_bound_and_is_not_taken() {
    let (hand, game) = debut_with_waitroom(&[]);
    assert!(
        !game.state.player1.waitroom.cards.is_empty(),
        "precondition: the waitroom still holds the two score-5 lives, so the pool was not \
         emptied by something else"
    );
    // THE assertion this test actually turns on. Without it the test passes for the wrong
    // reason: drop the score clause and the two score-5 lives both become candidates, the
    // engine opens a SelectCard, and the drain below never answers it — so "hand unchanged"
    // and "waitroom length 2" both hold anyway. No card moves not because the filter
    // refused, but because the selection is still pending. Zero candidates must be a SILENT
    // no-op, so no prompt may survive.
    assert!(
        !game.has_pending_choice(),
        "スコア6以上 matches neither live in the waitroom, so zero candidates qualify and the \
         recovery must resolve silently. A pending choice here means the score-5 lives were \
         offered as candidates — i.e. the スコア clause was dropped and the card would let the \
         player pick a sub-6 live."
    );
    assert!(
        !hand.iter().any(|c| c == SCORE_5),
        "スコア6以上 — a live printing 5 is below the bound and must not be recovered. \
         Reaching hand means the score clause was dropped and any live card qualifies. \
         Got hand card numbers {hand:?}"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        2,
        "both score-5 lives stay in the waitroom — スコア6以上 matches neither"
    );}

/// Above the bound qualifies, so the filter is a real threshold and not 「score 6 exactly」.
#[test]
fn a_live_of_score_seven_also_qualifies() {
    let (hand, _game) = debut_with_waitroom(&[SCORE_7]);
    assert!(
        hand.iter().any(|c| c == SCORE_7),
        "スコア6以上 — a live printing 7 is above the bound and must be recovered. \
         Got hand card numbers {hand:?}"
    );
}

/// The discriminating case: both a qualifying and a non-qualifying live in the zone at once.
///
/// With a correct filter only the score-6 card is a candidate, so `count: 1` resolves to it
/// exactly and NO prompt is raised. This is the assertion that fails when the clause is
/// dropped: the engine then has two candidates and opens a selection.
#[test]
fn only_the_qualifying_live_is_offered_when_both_are_in_the_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let six = game.id(SCORE_6);
    game.assert_card_identity(six, SCORE_6);
    game.assert_card_score(six, 6);
    let five = game.id(SCORE_5);
    game.assert_card_identity(five, SCORE_5);
    game.assert_card_score(five, 5);
    game.state.player1.waitroom.cards.push(six);
    game.state.player1.waitroom.cards.push(five);

    let nozomi = game.id(NOZOMI);
    game.assert_card_identity(nozomi, NOZOMI);
    game.add_to_hand(nozomi);
    game.give_energy(20);
    game.play_to_stage(nozomi, MemberArea::Center);
    game.drain_auto_ability_choices();

    assert!(
        !game.has_pending_choice(),
        "precondition: スコア6以上 matches exactly one of the two lives, so the recovery \
         resolves without asking. A prompt here means the score-5 live was also a candidate."
    );
    assert!(
        game.state.player1.hand.cards.contains(&six),
        "the score-6 live — the only one meeting スコア6以上 — reaches hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&five),
        "the score-5 live must be left in the waitroom"
    );
}
