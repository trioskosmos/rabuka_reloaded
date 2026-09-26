//! 桜小路きな子 `PL!SP-bp4-006-R` — the compound ライブ成功時 condition.
//!
//! 「エールにより公開された自分のカードの中に、名前が異なる『Liella!』のメンバーカードが
//! 3枚以上ある場合、エールにより公開された自分のカードの中から『Liella!』のライブカードを
//! 1枚手札に加える。」
//!
//! This is the tightest ライブ成功時 condition in the deck: a COUNT over DISTINCT
//! NAMES, a GROUP filter, and a second GROUP filter over a different card type for
//! what is actually retrieved. Each clause is independently capable of being wrong
//! and none of them is independently observable — a run that retrieves a live
//! proves all four at once, and a run that retrieves nothing proves none of them.
//!
//! The one existing test
//! (`live_success_three_distinct_revealed_retrieves_live_test.rs`) covers only the
//! four-clause-true case, and its third member id `PL!SP-bp1-014-PR` DOES NOT EXIST
//! — the lenient card-id fallback substituted a different print, so the "three
//! DISTINCT names" premise was never actually pinned. A distinct-name count is
//! exactly the claim a substituted card can silently invalidate, so the negatives
//! below are the only thing standing between this condition and a green suite that
//! proves nothing about distinctness.
//!
//! The revealed set is `GameState::revealed_cards`, the state the yell produces, so
//! each clause is driven directly and named.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

/// Three DIFFERENT 『Liella!』 members (all ラブライブ！スーパースター!! series),
/// verified to exist and to carry three different names.
const MEMBER_A: &str = "PL!SP-pb1-001-PR"; // 澁谷かのん
const MEMBER_B: &str = "PL!SP-bp1-004-PR"; // 平安名すみれ
const MEMBER_C: &str = "PL!SP-PR-005-PR"; // 嵐 千砂都
/// A 『Liella!』 LIVE card — the thing actually retrieved.
const LIELLA_LIVE: &str = "PL!SP-bp1-023-L";
const KEKE: &str = "PL!SP-bp4-006-R";

/// Stage きな子, then reveal the given cards as the yell would have.
fn stage_and_reveal(game: &mut TestGame, revealed: &[i16]) -> i16 {
    let keke = game.id(KEKE);
    game.assert_card_identity(keke, KEKE);
    game.state.player1.stage.stage[0] = keke;
    game.state.revealed_cards.clear();
    for card in revealed {
        game.state.revealed_cards.push(*card);
    }
    keke
}

/// The three distinct 『Liella!』 members, as verified ids. Premise-checked so a
/// card-pool edit that changes one of them names the failure instead of quietly
/// collapsing the distinctness the tests are about.
fn three_distinct_members(game: &mut TestGame) -> (i16, i16, i16) {
    let a = game.id(MEMBER_A);
    let b = game.id(MEMBER_B);
    let c = game.id(MEMBER_C);
    for (id, name) in [(a, MEMBER_A), (b, MEMBER_B), (c, MEMBER_C)] {
        game.assert_card_identity(id, name);
        game.assert_card_in_group(id, "Liella!", "the member must be inside 『Liella!』");
    }
    assert_ne!(
        game.db.get_card(a).unwrap().name,
        game.db.get_card(b).unwrap().name,
        "precondition: the first two members must have DIFFERENT names"
    );
    assert_ne!(
        game.db.get_card(b).unwrap().name,
        game.db.get_card(c).unwrap().name,
        "precondition: the second and third members must have DIFFERENT names"
    );
    assert_ne!(
        game.db.get_card(a).unwrap().name,
        game.db.get_card(c).unwrap().name,
        "precondition: the first and third members must have DIFFERENT names"
    );
    (a, b, c)
}

/// CLAUSE 1 — the threshold is 3枚以上, so two distinct names is not enough.
///
/// The count is over NAMES, so the sharpest version of this negative is two
/// distinct names PLUS extra copies of them: five revealed cards, two names. A
/// count-over-cards implementation retrieves here.
#[test]
fn two_distinct_names_retrieve_nothing_however_many_copies_are_revealed() {
    let mut game = TestGame::new(load_real_database());
    let (a, b, _c) = three_distinct_members(&mut game);
    let live = game.id(LIELLA_LIVE);
    game.assert_card_in_group(live, "Liella!", "the live is 『Liella!』 so only the member count is under test");

    // Five revealed cards, but only TWO distinct names.
    let revealed = [a, b, a, b, a];
    let keke = stage_and_reveal(&mut game, &revealed);
    let hand_before = game.state.player1.hand.cards.len();

    assert!(
        revealed.len() >= 3,
        "precondition: at least 3 CARDS are revealed, so a count-over-cards \
         implementation would pass this test — it is a count-over-names test"
    );
    let distinct: std::collections::HashSet<String> = revealed
        .iter()
        .map(|id| game.db.get_card(*id).unwrap().name.to_string())
        .collect();
    assert_eq!(
        distinct.len(),
        2,
        "precondition: the revealed set must contain exactly 2 distinct names \
         (got {distinct:?})"
    );

    fire_trigger(&mut game, keke, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "『名前が異なる『Liella!』のメンバーカードが3枚以上』 counts NAMES: 5 cards \
         across 2 names is 2, so nothing is retrieved and the revealed live stays put"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "the 『Liella!』 live must not reach hand — it is only retrieved once the \
         NAME threshold is met"
    );
}

/// CLAUSE 2 — a non-『Liella!』 member does not count toward the total.
///
/// The group filter and the count are separate clauses, and a count that ignored
/// the group would pass the happy path with the wrong cards. Three DIFFERENT names,
/// all outside 『Liella!``, so nothing but the group filter can produce the failure.
#[test]
fn three_names_that_are_not_liella_retrieve_nothing() {
    let mut game = TestGame::new(load_real_database());
    // 乙宗梢 (蓮ノ空), 上原歩夢 (虹ヶ咲), 国木田花丸 (サンシャイン) — three series,
    // three names, none of them 『Liella!』.
    let outsiders = [
        ("PL!HS-bp1-012-PR", "乙宗梢"),
        ("PL!N-bp1-001-R", "上原歩夢"),
        ("PL!S-bp6-007-R", "国木田花丸"),
    ];
    let mut ids = Vec::new();
    for (card_no, who) in outsiders {
        let id = game.id(card_no);
        game.assert_card_identity(id, card_no);
        assert!(
            !rabuka_engine::ability::util::card_matches_group_str(
                &game.db,
                id,
                Some("Liella!")
            ),
            "precondition: {who} must be OUTSIDE 『Liella!』, or the group filter is \
             not exercised in the negative direction"
        );
        ids.push(id);
    }
    let live = game.id(LIELLA_LIVE);
    let mut revealed = ids.clone();
    revealed.push(live);
    let keke = stage_and_reveal(&mut game, &revealed);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, keke, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "three non-『Liella!』 member names do not satisfy 『『Liella!』のメンバーカードが \
         3枚以上』 — the group filter is part of the count, not decoration"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "and the 『Liella!』 live stays revealed: the count clause gates the \
         retrieval, so a Liella! live alone is not enough"
    );
}

/// CLAUSE 3 — the two clauses are INDEPENDENT: the count can hold while the
/// retrievable set is empty.
///
/// This is the case a single combined test cannot produce, and it is where an
/// implementation that filters the whole revealed set by 『Liella!』 and then takes
/// "the first one" would retrieve a MEMBER instead of a LIVE card.
#[test]
fn threshold_met_but_no_liella_live_revealed_retrieves_no_card_at_all() {
    let mut game = TestGame::new(load_real_database());
    let (a, b, c) = three_distinct_members(&mut game);
    let keke = stage_and_reveal(&mut game, &[a, b, c]);
    let hand_before = game.state.player1.hand.cards.len();
    let revealed_before = game.state.revealed_cards.len();

    fire_trigger(&mut game, keke, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "the member clause is satisfied but there is no 『Liella!』 ライブカード among \
         the revealed cards, so NOTHING is retrieved — a member must not be taken \
         in its place"
    );
    assert_eq!(
        game.state.revealed_cards.len(),
        revealed_before,
        "the revealed set is left intact when the second clause finds nothing"
    );
}

/// CLAUSE 4 — the positive, with the name count and the group filter both pinned,
/// so the negatives above have a control that would fail if the ability stopped
/// working entirely.
#[test]
fn three_distinct_liella_names_plus_a_liella_live_retrieves_exactly_that_live() {
    let mut game = TestGame::new(load_real_database());
    let (a, b, c) = three_distinct_members(&mut game);
    let live = game.id(LIELLA_LIVE);
    let keke = stage_and_reveal(&mut game, &[a, b, c, live]);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, keke, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "control: all four clauses hold, so exactly one card is retrieved"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "and the card retrieved is the revealed 『Liella!』 live, not a member"
    );
}
