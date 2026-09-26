//! Group membership: the engine infers it from `series`, but `unit` is the field
//! that actually names a group.
//!
//! `Card` carries both (`engine/src/core/card.rs:322-323`): `group` (empty in
//! the card data) and `unit`. `card_matches_group_str` falls back through
//! `unit == group_name`, then the absent `group`, then name fragments, then
//! `card_series_matches_group` (`engine/src/ability/util.rs`). That last arm
//! decides the グループ名称 filters — μ's / Aqours / 虹ヶ咲 / Liella! / 蓮ノ空 —
//! purely by looking for another group's marker inside the `series` STRING.
//!
//! `engine/rules/rules.txt` is where the truth lives, and it lists the two
//! vocabularies separately:
//!
//!   グループ名称: μ's, Aqours, 虹ヶ咲, Liella!, 蓮ノ空, A-RISE, Saint Snow,
//!                Sunny Passion
//!   ユニット名称: Printemps, BiBi, lily white, CYaRon！, AZALEA, Guilty Kiss,
//!                QU4RTZ, A・ZU・NA, DiverDiva, R3BIRTH, CatChu!, KALEIDOSCORE,
//!                5yncri5e!, スリーズブケ, DOLLCHESTRA, みらくらぱーく！,
//!                Edel Note, AiScReam
//!
//! A `series` value is a set/series label, not a group label, so the arm
//! misjudges in both directions. Measured over `cards/cards.json`, counting a
//! card as "in group G" when its `unit` is one of G's ユニット名称:
//!
//! | group        | cards in unit | engine matches | invisible | crossover |
//! |--------------|---------------|----------------|-----------|-----------|
//! | μ's          | 215           | 119            | 0         | 96        |
//! | Aqours       | 334           | 127            | 207       | 0         |
//! | 虹ヶ咲        | 575           | 323            | 0         | 252       |
//! | Liella!      | 724           | 511            | 0         | 213       |
//! | 蓮ノ空        | 142           | 142            | 0         | 0         |
//! | Sunny Passion| 1             | 0              | 1         | 0         |
//!
//! ("crossover" = the series also names another group, i.e. a collaboration card
//! that the arm attributes to only one of its groups — expected for now.)
//!
//! Three gaps, all the same root cause. Every assertion below pins the CURRENT
//! answer, so a fix in the matcher turns them red and they can be rewritten as
//! the real rule. Nothing here touches the engine.

use crate::helpers::*;
use rabuka_engine::ability::util::card_matches_group_str;

/// The same card, asked the same question two ways.
#[test]
fn group_membership_known_gap_aqours_card_matches_its_unit_but_not_its_group() {
    let db = load_real_database();
    let game = TestGame::new(db);

    // 高坂 穂乃果 is not in this group; 絢瀬 絵里 is BiBi — an Aqours unit.
    let bibi = game.id("PL!-sd1-002-SD");
    assert_eq!(
        game.db.get_card(bibi).unwrap().unit.as_deref(),
        Some("BiBi"),
        "fixture: this card's unit is a ユニット名称 of Aqours"
    );

    // The unit path works: the ability text 「『BiBi』のカード」 resolves.
    assert!(
        card_matches_group_str(&game.db, bibi, Some("BiBi")),
        "『BiBi』 matches on unit — the unit path is sound"
    );
    // The group path does not: this card's `series` is plain 「ラブライブ！」 with
    // no サンシャイン marker, and `card_series_matches_group(series, "Aqours")`
    // is `series.contains("サンシャイン")`. So 「『Aqours』のカード」 finds
    // nothing — 207 of Aqours' 334 cards.
    assert!(
        !card_matches_group_str(&game.db, bibi, Some("Aqours")),
        "KNOWN GAP: the SAME card answers yes to 『BiBi』 and no to 『Aqours』 — \
         group membership is read off `series`, which for this card is plain \
         「ラブライブ！」"
    );
}

#[test]
fn group_membership_known_gap_muren_card_is_invisible_to_mus_filter() {
    let db = load_real_database();
    let game = TestGame::new(db);

    // ド！ド！ド！ — a μ's LIVE, unit みらくらぱーく! (half-width ! here; the
    // other μ's cards spell it みらくらぱーく！, and `norm_group_name` folds
    // ！→! for exactly that reason). μ's renamed their UNIT, not their school,
    // so the card prints in the 蓮ノ空 series, and the μ's arm rejects any
    // series containing 蓮ノ空. 96 of μ's' 215 cards are like this.
    let muren = game.id("PL!HS-bp1-023-L");
    assert_eq!(
        game.db.get_card(muren).unwrap().unit.as_deref(),
        Some("みらくらぱーく!"),
        "fixture: みらくらぱーく！ is one of μ's ユニット名称"
    );
    assert!(
        card_matches_group_str(&game.db, muren, Some("みらくらぱーく!")),
        "the unit name itself resolves"
    );
    assert!(
        !card_matches_group_str(&game.db, muren, Some("μ's")),
        "KNOWN GAP: a μ's card that no 『μ's』 filter can see, because its \
         series is ラブライブ！蓮ノ空女学院スクールアイドルクラブ"
    );

    // The control that makes the gap specific rather than universal: the same
    // filter DOES see μ's' other-era cards, whose series is plain ラブライブ！.
    let printemps = game.id("PL!-sd1-001-SD");
    assert_eq!(
        game.db.get_card(printemps).unwrap().unit.as_deref(),
        Some("Printemps"),
        "fixture: Printemps is the other μ's unit name"
    );
    assert!(
        card_matches_group_str(&game.db, printemps, Some("μ's")),
        "precondition: the μ's filter is not broken for everyone — a \
         plain-ラブライブ！ μ's card is matched"
    );
}

#[test]
fn group_membership_known_gap_sunny_passion_filter_is_unsatisfiable() {
    let db = load_real_database();
    let game = TestGame::new(db);

    // AiScReam is Sunny Passion's unit. `card_series_matches_group` has arms for
    // μ's / Aqours / 虹ヶ咲 / Liella! / 蓮ノ空 and `_ => false` for everything
    // else, so a 『Sunny Passion』 filter has no series path at all — and the
    // `unit == group` path compares "AiScReam" with "Sunny Passion", which is
    // not equal either. Nothing can satisfy it.
    let ai_scream = game.id("LL-PR-004-PR");
    assert_eq!(
        game.db.get_card(ai_scream).unwrap().unit.as_deref(),
        Some("AiScReam"),
        "fixture: AiScReam is Sunny Passion's ユニット名称"
    );
    assert!(
        !card_matches_group_str(&game.db, ai_scream, Some("Sunny Passion")),
        "KNOWN GAP: no arm in card_series_matches_group handles Sunny Passion, \
         so the filter can never be true"
    );
    assert!(
        card_matches_group_str(&game.db, ai_scream, Some("AiScReam")),
        "…while the unit name itself does match"
    );
}

/// Groups whose filters are NOT broken, so the three gaps above read as gaps and
/// not as "group filtering never worked".
#[test]
fn group_membership_works_for_the_three_covered_groups() {
    let db = load_real_database();
    let game = TestGame::new(db);

    for (card_no, group, note) in [
        ("PL!HS-bp1-001-R", "蓮ノ空", "スリーズブケ member, 蓮ノ空 series"),
        ("PL!N-sd1-001-SD", "虹ヶ咲", "A・ZU・NA member, 虹ヶ咲 series"),
        ("PL!SP-pb2-025-P", "Liella!", "Liella! live, スーパースター series"),
    ] {
        let id = game.id(card_no);
        assert!(
            card_matches_group_str(&game.db, id, Some(group)),
            "{} must be matched by a 『{}』 filter ({})",
            card_no,
            group,
            note
        );
    }
}
