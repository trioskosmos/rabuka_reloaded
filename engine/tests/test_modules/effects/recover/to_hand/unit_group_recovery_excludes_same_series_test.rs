//! The two routes through `card_matches_group_str`, which a dozen card filters in
//! this deck depend on and which nothing in the suite separated.
//!
//! `card_matches_group_str` resolves a group name two ways: by matching the card's
//! SERIES against the five series-level names (`μ's`, `Aqours`, `虹ヶ咲`, `Liella!`,
//! `蓮ノ空`), and by exact match against the card's UNIT. Every unit-level name
//! relies on the second route alone, and the two routes disagree on cards that
//! share a series — which is most of the pool.
//!
//! Three card filters turn on the distinction:
//!
//!   * 若菜四季 `PL!SP-bp2-019-N` — 「自分の控え室から**『5yncri5e!』**のカードを1枚手札に
//!     加える」. 『5yncri5e!』 is a UNIT, not a series, so a series-based reading
//!     would also recover 平安名すみれ `PL!SP-bp1-004-PR` — same
//!     ラブライブ！スーパースター!! series, unit `CatChu!`.
//!   * 平安名すみれ `PL!SP-bp2-015-N` and ウィーン `PL!SP-bp2-021-N` — the same
//!     printed shape with 『CatChu!』 and 『KALEIDOSCORE』, also unit-level names.
//!   * 米女メイ `PL!SP-bp2-007-R` — 『Liella!』, a SERIES-level name, which matches
//!     cards with no unit at all.
//!
//! So the test below asserts the rule at the level it is defined, on three cards
//! that share a series: two statements that could each be reached by accident if
//! only one route existed, and one that pins which route each name uses.
//!
//! ## Still uncovered here
//!
//! The end-to-end recovery tests for the three cards above are NOT included. Their
//! optional cost — 「手札を1枚控え室に置いてもよい」 — did not present a prompt under
//! `play_to_stage` with the card staged this way, so the tests would have been
//! measuring a fixture rather than the filter. The filter itself, which is the part
//! that could silently be wrong, is pinned here instead.

use crate::helpers::*;

const FIVEYNCR: &str = "PL!SP-bp1-014-N"; // 嵐 千砂都 — unit 5yncri5e!
const CATCHU: &str = "PL!SP-bp1-004-PR"; // 平安名すみれ — unit CatChu!, SAME series

/// The two routes through `card_matches_group_str`, separated on cards that share
/// a series so neither conclusion can be reached by accident.
///
/// `card_matches_group_str` resolves a group name two ways: by matching the card's
/// SERIES against the five series-level names, and by exact match against the
/// card's UNIT. The three statements below are what a filter on either name
/// depends on, and nothing else in the suite states all three together:
///
///   * a card with `unit: None` still matches a SERIES-level name — the only route
///     available to it, since there is no unit to match;
///   * a card with a DIFFERENT unit still matches that same series-level name,
///     because the series route does not care what the unit is;
///   * and neither of them matches a UNIT-level name like 『5yncri5e!』, which is
///     the whole reason a 5yncri5e! filter is the sharper of the two.
///
/// The middle statement is the one that is easy to get backwards, and it is why
/// 『5yncri5e!』 is the sharper probe: 平安名すみれ and 嵐 千砂都 share a series, so a
/// series-based reading of 『5yncri5e!』 would recover her and no amount of
/// series-level filtering would tell the difference.
#[test]
fn series_level_and_unit_level_groups_resolve_by_different_routes() {
    let game = TestGame::new(load_real_database());
    let unitless_live = game.id("PL!SP-bp1-023-L"); // Superstar live, unit: None
    let catchu = game.id(CATCHU); // Superstar member, unit: CatChu!
    let five = game.id(FIVEYNCR); // Superstar member, unit: 5yncri5e!
    game.assert_card_identity(unitless_live, "PL!SP-bp1-023-L");
    game.assert_card_identity(catchu, CATCHU);
    game.assert_card_identity(five, FIVEYNCR);

    let card = |id: i16| game.state.card_database.get_card(id).unwrap();
    let matches =
        |id: i16, group: &str| rabuka_engine::ability::util::card_matches_group_str(&game.db, id, Some(group));

    assert_eq!(
        card(unitless_live).unit,
        None,
        "precondition: this live must print NO unit, or the series route is not the \
         only thing that can match it"
    );
    assert!(
        matches(unitless_live, "Liella!"),
        "a card with no unit still matches a SERIES-level group name — 『Liella!』 is \
         one of the five series-level names, so this is the route 『5yncri5e!』 cannot \
         use"
    );
    assert!(
        matches(catchu, "Liella!"),
        "and a card whose unit is something else entirely STILL matches it: the series \
         route does not consult the unit. An earlier draft of this test asserted the \
         opposite, which is what made 『5yncri5e!』 worth isolating — the two names \
         differ exactly here."
    );
    assert!(
        !matches(catchu, "5yncri5e!") && !matches(unitless_live, "5yncri5e!"),
        "neither matches a UNIT-level name: 『5yncri5e!』 is not one of the five \
         series-level groups, so only an exact unit match can satisfy it, and neither \
         card's unit is 5yncri5e!"
    );
    assert!(
        matches(five, "5yncri5e!"),
        "while a card whose unit IS 5yncri5e! does match it — same series as the two \
         above, so this pair is what shows the route is the unit and not the series"
    );
}
