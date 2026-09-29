//! Group-name matching ignores spacing.
//!
//! The corpus spells one unit group two ways: `lily white` on some cards and
//! `lilywhite` on others — e.g. `PL!-pb1-029-SRL` 知らないLove (spaced) against
//! `PL!-PR-004-PR` 園田海未 (unspaced). A raw `==` in `card_matches_group_str` meant a
//! `『lily white』` filter silently excluded every unspaced card, so any ability
//! gated on that group under-triggered.
//!
//! These cases pin the normalisation and, just as importantly, pin what it must NOT
//! merge: a different group, and a group that differs by more than spacing.

use crate::helpers::*;

const SPACED: &str = "PL!-pb1-029-SRL"; // 知らないLove — unit "lily white"
const UNSPACED: &str = "PL!-PR-004-PR"; // 園田海未 — unit "lilywhite"
const BIBI: &str = "PL!-pb2-002-PP"; // 絢瀬絵里 — unit "BiBi"
const AZALEA: &str = "PL!S-bp5-012-N"; // 松浦果南 — unit "AZALEA"
/// Substring-overlap trap: "BiBi" is not a prefix/suffix of "BiBiS" or similar, but
/// a naive "contains" comparison would wrongly merge groups that merely share a run
/// of letters. This card's unit starts with the same letters as none of the others.
const CATCHU: &str = "PL!SP-bp4-004-P"; // 平安名すみれ — unit "CatChu!"

#[test]
fn the_two_spellings_are_the_same_group() {
    let db = load_real_database();
    let game = TestGame::new(db);
    for card_no in [SPACED, UNSPACED] {
        let id = game.id(card_no);
        let unit = game
            .db
            .get_card(id)
            .unwrap()
            .unit
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();
        assert!(
            unit.contains("lily") && unit.replace(' ', "").eq_ignore_ascii_case("lilywhite"),
            "fixture precondition: {card_no} prints unit {unit:?}"
        );
        game.assert_card_in_group(id, "lily white", "spaced spelling");
        game.assert_card_in_group(id, "lilywhite", "unspaced spelling");
    }
}

#[test]
fn an_unspaced_card_matches_the_spaced_group_name() {
    let db = load_real_database();
    let game = TestGame::new(db);
    // The case that was broken: 園田海未 (lilywhite) under a 『lily white』 filter.
    game.assert_card_in_group(
        game.id(UNSPACED),
        "lily white",
        "unspaced unit under a spaced group name",
    );
}

#[test]
fn a_spaced_card_matches_the_unspaced_group_name() {
    let db = load_real_database();
    let game = TestGame::new(db);
    game.assert_card_in_group(
        game.id(SPACED),
        "lilywhite",
        "spaced unit under an unspaced group name",
    );
}

#[test]
fn a_different_group_still_does_not_match() {
    let db = load_real_database();
    let game = TestGame::new(db);
    // Normalising spacing must not make the check fuzzy about everything else.
    for (card_no, wrong_group) in [(BIBI, "lily white"), (AZALEA, "lily white"), (CATCHU, "lily white")] {
        let id = game.id(card_no);
        assert!(
            !rabuka_engine::ability::util::card_matches_group_str(
                &game.db,
                id,
                Some(wrong_group)
            ),
            "{card_no} is not a lily white member and must not match {wrong_group:?}"
        );
    }
}

#[test]
fn a_longer_group_is_not_collapsed_onto_a_shorter_one() {
    let db = load_real_database();
    let game = TestGame::new(db);
    // A group that merely CONTAINS the other as a substring must not match: this is
    // why the comparison is equality-after-stripping, not `contains`.
    for (card_no, other) in [(BIBI, "lil"), (CATCHU, "Cat")] {
        let id = game.id(card_no);
        assert!(
            !rabuka_engine::ability::util::card_matches_group_str(&game.db, id, Some(other)),
            "{card_no} must not match the partial group {other:?}"
        );
    }
}
