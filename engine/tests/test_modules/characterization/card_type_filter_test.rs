//! `card_matches_type_for` — the card-type filter must never widen by accident.
//!
//! The catch-all used to be `_ => true`, shared with the `None` arm. That conflated
//! "no type was asked for" (everything matches, correctly) with "I do not recognise
//! this type string" (everything matches, wrongly). An unknown filter that matches
//! every card produces a plausible game state that no assertion can tell from a
//! correct one, and it fails open rather than closed, which is the dangerous
//! direction: a wrong target is harder to notice than a missing one.
//!
//! PL!N-bp7-028-L Cooking with Love is the case that reached it —
//! 「自分のステージにいるすべてのグループのメンバーはheart01を得る」 parsed to
//! `card_type: "member"`, took the catch-all, and applied to anything on the stage.
//!
//! The spelling set is wider than the parser's canonical vocabulary because the
//! value also comes from engine-synthesised filters and test fixtures: `member`,
//! `Member` and `""` were all observed in the run that first made this fail closed.
//! Hence normalisation rather than an enumeration.

use crate::helpers::*;
use rabuka_engine::ability::util::card_matches_type_for;
use rabuka_engine::card::CardType;

fn a_card(game: &TestGame, card_no: &str, want: CardType) -> rabuka_engine::core::card::Card {
    let card = game.db.get_card(game.id(card_no)).expect("fixture card").clone();
    assert_eq!(card.card_type, want, "{card_no} fixture has the expected type");
    card
}

/// Each canonical type matches only itself, and 「カード」 plus an absent filter
/// match everything.
#[test]
fn each_type_filter_matches_only_its_own_type() {
    let game = TestGame::new(load_real_database());
    let member = a_card(&game, "PL!-pb2-001-R", CardType::Member);
    let energy = a_card(&game, "LL-E-001-PR", CardType::Energy);
    let live = a_card(&game, "PL!-pb1-029-SRL", CardType::Live);

    let cases: &[(&str, &str, bool)] = &[
        // filter, card, expected
        ("member_card", "member", true),
        ("member_card", "energy", false),
        ("member_card", "live", false),
        ("live_card", "live", true),
        ("live_card", "member", false),
        ("energy_card", "energy", true),
        ("energy_card", "member", false),
        // 「カード」 and "no filter asked" both mean no restriction.
        ("card", "member", true),
        ("card", "energy", true),
        ("card", "live", true),
    ];

    for (filter, which, expected) in cases {
        let card: &rabuka_engine::core::card::Card = match *which {
            "member" => &member,
            "energy" => &energy,
            _ => &live,
        };
        assert_eq!(
            card_matches_type_for(Some(card), Some(filter)),
            *expected,
            "card_type {filter:?} against a {which} card"
        );
    }

    assert!(
        card_matches_type_for(Some(&member), None),
        "no type filter means nothing is being asked, so nothing can fail to match"
    );
}

/// Spelling variants must reach the same verdict as the canonical form.
///
/// A bare 「メンバー」 with no 「カード」 beside it parses to `member`, and the
/// capitalised `Member` comes from a filter assembled outside the parser. None of
/// them may fall through.
#[test]
fn type_filter_spellings_do_not_change_the_verdict() {
    let game = TestGame::new(load_real_database());
    let member = a_card(&game, "PL!-pb2-001-R", CardType::Member);
    let energy = a_card(&game, "LL-E-001-PR", CardType::Energy);

    for filter in [
        "member",
        "Member",
        "MEMBER",
        " member ",
        "member_card",
        "MEMBER_CARD ",
    ] {
        assert!(
            card_matches_type_for(Some(&member), Some(filter)),
            "{filter:?} is a member filter and must match a member"
        );
        assert!(
            !card_matches_type_for(Some(&energy), Some(filter)),
            "{filter:?} is a member filter and must NOT match an energy card"
        );
    }

    // An empty string is a filter that was populated but never filled in — the same
    // statement as 「カード」, and no restriction.
    for filter in ["", "  ", "card", "Card"] {
        assert!(
            card_matches_type_for(Some(&energy), Some(filter)),
            "{filter:?} states no restriction and must match anything"
        );
    }
}

/// An unrecognised filter matches nothing, rather than everything.
#[test]
fn an_unrecognized_type_filter_matches_nothing() {
    let game = TestGame::new(load_real_database());
    let member = a_card(&game, "PL!-pb2-001-R", CardType::Member);
    let energy = a_card(&game, "LL-E-001-PR", CardType::Energy);
    let live = a_card(&game, "PL!-pb1-029-SRL", CardType::Live);

    for filter in ["memberz", "tokyo", "member card", "support_card", "card_type"] {
        for card in [&member, &energy, &live] {
            assert!(
                !card_matches_type_for(Some(card), Some(filter)),
                "{filter:?} is not a type this engine knows; it must not match \
                 {} — \"I do not know what this means\" is not \"it matches\"",
                card.name
            );
        }
    }

    // A card that is not in the database cannot be shown to be of the requested
    // type, so a real filter rejects it and an unrestricted one accepts it.
    let missing = None;
    assert!(!card_matches_type_for(missing, Some("member_card")));
    assert!(card_matches_type_for(missing, Some("card")));
    assert!(card_matches_type_for(missing, None));
}
