//! Equivalence guard for the allocation-free name-part group predicate.
//!
//! `ability::util::card_name_matches_group` is the fast path inside
//! `card_matches_group_str`'s fourth disjunct. It skips
//! `CardDatabase::get_card_names` (4+ heap allocations per candidate card) when
//! the card name contains no whitespace and no full-width '＆', because in that
//! case `get_card_names` provably reduces to `name.split('&')`.
//!
//! That reasoning is only sound if it holds for the WHOLE corpus, including the
//! multi-name cards and the `lily white`/`lilywhite` spelling split the corpus is
//! known to use. If the fast-path condition is ever wrong, a card silently stops
//! matching a group filter — which changes which plays the engine offers, i.e.
//! gameplay, and would not necessarily fail a behavioural test that happened not
//! to cover the affected card. So this asserts the fast path and the allocating
//! path agree, exhaustively, over the real card database.

use rabuka_engine::ability::util::{card_name_matches_group, KNOWN_GROUPS};
use rabuka_engine::card::CardDatabase;

use crate::helpers::load_real_database;

/// Mirror of the private `ability::util::norm_group_name`, used only by the
/// oracle so the reference stays faithful to the original implementation.
fn normalize_fullwidth(s: &str) -> String {
    let has_ff01 = s.contains('\u{FF01}');
    let has_mu = s.contains('\u{00B5}');
    if has_ff01 || has_mu {
        let mut r = if has_ff01 {
            s.replace('\u{FF01}', "!")
        } else {
            s.to_string()
        };
        if has_mu {
            r = r.replace('\u{00B5}', "\u{03BC}");
        }
        r
    } else {
        s.to_string()
    }
}

/// The ORIGINAL, always-allocating implementation, kept here as the oracle:
/// byte-for-byte the pre-optimisation disjunct from `card_matches_group_str`.
fn reference_name_matches_group(db: &CardDatabase, card_id: i16, gn: &str) -> bool {
    db.get_card_names(card_id).iter().any(|n| {
        n.contains(gn)
            || ((n.contains('\u{FF01}') || n.contains('\u{00B5}'))
                && normalize_fullwidth(n).contains(gn))
    })
}

/// Group names to probe with: the canonical list plus needles that exercise the
/// full-width folds, the `&` multi-name split, and a non-matching control.
fn probe_groups() -> Vec<String> {
    let mut out: Vec<String> = KNOWN_GROUPS.iter().map(|s| s.to_string()).collect();
    for extra in [
        "！",
        "µ",
        "μ",
        "lily white",
        "lilywhite",
        "A",
        "nosuchgroupanywhere",
    ] {
        out.push(extra.to_string());
    }
    out
}

#[test]
fn fast_path_matches_allocating_path_for_whole_corpus() {
    let db = load_real_database();
    let groups = probe_groups();

    let mut checked = 0usize;
    for (card_id, card) in db.iter_cards() {
        for g in &groups {
            let fast = card_name_matches_group(&db, card, card_id, g);
            let slow = reference_name_matches_group(&db, card_id, g);
            assert_eq!(
                fast, slow,
                "group-name mismatch for card {} ({:?}) against group {:?}",
                card_id,
                card.name.as_ref(),
                g
            );
            checked += 1;
        }
    }
    assert!(
        checked > 1000,
        "expected to sweep the whole corpus, only checked {} (card, group) pairs",
        checked
    );
}

/// The specific premise the optimisation rests on: the fast path's
/// short-circuit condition is exactly "no whitespace and no full-width '＆'",
/// and in that case `get_card_names` really does reduce to `split('&')`.
#[test]
fn fast_path_precondition_matches_get_card_names_semantics() {
    let db = load_real_database();
    let mut verified = 0usize;
    for (card_id, card) in db.iter_cards() {
        let name: &str = card.name.as_ref();
        let has_ws = name.contains(char::is_whitespace);
        let has_fullwidth_amp = name.contains('＆');
        if has_ws || has_fullwidth_amp {
            continue;
        }
        let parts: Vec<String> = CardDatabase::normalize_name(name)
            .replace('＆', "&")
            .split('&')
            .map(|s| s.to_string())
            .collect();
        let borrowed_parts: Vec<&str> = name.split('&').collect();
        assert_eq!(
            parts, borrowed_parts,
            "fast-path split diverged from get_card_names for card {}",
            card_id
        );
        verified += 1;
    }
    assert!(
        verified > 1000,
        "expected most of the corpus to take the fast path, only {} did",
        verified
    );
}

/// Multi-name cards are the reason `get_card_names` exists at all, so make sure
/// the corpus still contains them — otherwise the tests above are vacuous and
/// the fast path is untested against the case it was written for.
#[test]
fn multi_name_cards_are_covered() {
    let db = load_real_database();
    let multi: Vec<&str> = db
        .cards
        .iter()
        .flatten()
        .map(|c| c.name.as_ref())
        .filter(|n| n.contains('&') || n.contains('＆'))
        .collect();
    assert!(
        !multi.is_empty(),
        "expected multi-name cards in the corpus; if the card data changed, \
         re-verify the fast-path condition in card_name_matches_group"
    );
}
