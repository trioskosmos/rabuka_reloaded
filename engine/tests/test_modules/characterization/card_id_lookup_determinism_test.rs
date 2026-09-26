use crate::helpers::*;

/// `CardDatabase::get_card_id` is lenient: an unknown rarity suffix falls back to
/// "any print of this card number". That fallback used to return the FIRST match
/// found by iterating a HashMap, whose order Rust randomises per process — so the
/// same `card_no` could resolve to a different print on different runs.
///
/// This matters well beyond tests: it is library code on the lookup path for
/// every card the game resolves, and a bot, a replay or a save that names a
/// print loosely would get a different card each time it started.
///
/// The contract now is: the lowest matching key wins, every time.
#[test]
fn card_id_unknown_rarity_fallback_is_deterministic() {
    let db = load_real_database();

    // A card number with several prints, requested with a rarity that does not
    // exist for it, so the lenient fallback has to do the work.
    let requested = "PL!SP-bp2-006-R";
    assert!(
        db.get_card_id(requested).is_some(),
        "fixture: {requested} must resolve through some fallback"
    );

    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..64 {
        let id = db
            .get_card_id(requested)
            .expect("the fallback always finds a print");
        let no = db.get_card(id).unwrap().card_no.to_string();
        // The fallback must return a real print of the SAME card number.
        assert!(
            no.starts_with("PL!SP-bp2-006-"),
            "the fallback must stay within the requested card number, got {no}"
        );
        seen.insert(no);
    }
    assert_eq!(
        seen.len(),
        1,
        "{requested} resolved to {seen:?} across 64 calls — the fallback must be \
         deterministic"
    );
}

/// A print that DOES exist must always win over the lenient fallback, and an
/// exact match must never be shadowed by another print of the same character.
#[test]
fn card_id_exact_and_equivalent_prints_are_not_shadowed() {
    let db = load_real_database();
    // Prints confirmed to exist (see cards.json): note HS/Nijigasaki KALEIDOSCORE
    // numbers use the `pb1-003` base, not `bp1-003`.
    for no in [
        "PL!SP-bp2-006-P",
        "PL!SP-bp2-006-SEC",
        "PL!S-pb1-003-R",
        "PL!S-pb1-003-P＋",
        "PL!HS-pb1-003-R",
    ] {
        let id = db
            .get_card_id(no)
            .unwrap_or_else(|| panic!("{no} must exist"));
        let actual = db.get_card(id).unwrap().card_no.to_string();
        assert_eq!(actual, no, "{no} resolved to a different print");
    }
}
