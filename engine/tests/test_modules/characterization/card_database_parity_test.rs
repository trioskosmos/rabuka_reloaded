//! Every card in `cards/cards.json` must resolve in the baked card database.
//!
//! # Why this exists
//!
//! Two generated Rust artefacts are verified fresh by CI: `abilities_gen.rs`
//! (via `compile_abilities.py`) and the two decoder files. `cards_gen.rs` is
//! gitignored and rebuilt from `cards.json` on every deploy, and the card blob
//! `cards/build/cards.bin.z` IS tracked but nothing re-derives it.
//!
//! So a card added to or removed from `cards.json` without re-running
//! `compile_cards.py` is invisible to CI. The engine keeps serving the old blob,
//! the parser reads the new JSON, and the two disagree — which is exactly what
//! `engine/tests/test_modules/perf/alloc_profile.rs` does today: it asks for
//! `PL!N-bp1-002-R`, which resolves against the baked blob but is not a key in
//! `cards/cards.json`. `test_inventory.py` flags it as `unresolvable_card_id`,
//! and nothing fails.
//!
//! This is that failure made loud. It asks the authoritative question — does the
//! database the tests actually run against know this card? — rather than
//! comparing text, which is unreliable because the blob interleaves card numbers
//! with ability strings.

use crate::helpers::*;

/// The database must know every card the source data declares.
///
/// Only `PL!` cards are baked into the blob; `cards.json` also holds cards for
/// other releases (`LL-*`, `IKZL-*`) that are deliberately not compiled in. Those
/// are excluded by prefix rather than by an allow-list of ids, so a new foreign
/// prefix does not need touching here.
#[test]
fn every_pl_card_in_cards_json_resolves_in_the_baked_database() {
    let db = load_real_database();

    let cards_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cards/cards.json");
    let raw = std::fs::read_to_string(&cards_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", cards_path.display()));
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("cards.json is not JSON: {e}"));
    let obj = parsed
        .as_object()
        .unwrap_or_else(|| panic!("cards.json must be an object keyed by card number"));

    let mut checked = 0usize;
    let mut unresolved: Vec<&str> = Vec::new();
    for card_no in obj.keys() {
        if !card_no.starts_with("PL!") {
            continue;
        }
        checked += 1;
        if db.get_card_id(card_no).is_none() {
            unresolved.push(card_no);
        }
    }

    assert!(
        unresolved.is_empty(),
        "{} of {checked} `PL!` cards in cards/cards.json are absent from the baked \
         database the engine tests run against. The blob is stale — re-run \
         `python cards/compile_cards.py`. First offenders: {:?}",
        unresolved.len(),
        &unresolved[..unresolved.len().min(12)]
    );
    assert!(
        checked > 2000,
        "sanity: most of the pool should be PL! cards, checked {checked}"
    );
}

/// The reverse direction: the database must not carry cards the source dropped.
///
/// A removed card that survives in the blob is worse than a missing one — it
/// looks selectable, resolves to a real id, and behaves in every way like a card
/// that no longer exists. `alloc_profile.rs` is currently living inside this
/// hazard, which is why it is worth pinning rather than merely noting.
#[test]
fn the_database_carries_no_cards_that_cards_json_dropped() {
    let db = load_real_database();

    let cards_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cards/cards.json");
    let raw = std::fs::read_to_string(&cards_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", cards_path.display()));
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("cards.json is not JSON: {e}"));
    let obj = parsed.as_object().expect("cards.json is an object");

    // Collect the card numbers the database reports, then keep only those shaped
    // like a card number. The blob interleaves ability text, so the filter is by
    // shape rather than by hoping the whole blob is card numbers.
    //
    // Ids are scanned over a bounded range because CardDatabase exposes no count;
    // 6000 is the same bound `action_coverage_test` uses and sits well above the
    // pool, and `get_card` returning `None` past the end makes the tail free.
    let mut extra: Vec<String> = Vec::new();
    let mut seen = 0usize;
    for id in 0..6000i16 {
        let Some(card) = db.get_card(id) else { continue };
        let card_no = card.card_no.as_ref();
        if !card_no.starts_with("PL!") {
            continue;
        }
        seen += 1;
        if !obj.contains_key(card_no) {
            extra.push(card_no.to_string());
        }
    }
    extra.sort();
    extra.dedup();

    assert!(
        extra.is_empty(),
        "{} card numbers are served by the baked database but are not keys in \
         cards/cards.json — the blob predates a removal. Re-run \
         `python cards/compile_cards.py`. Offenders: {:?}",
        extra.len(),
        &extra[..extra.len().min(15)]
    );
    assert!(seen > 2000, "sanity: checked {seen} baked PL! cards");
}
