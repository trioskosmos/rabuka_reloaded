use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const KANON: &str = "PL!SP-pb1-001-PR"; // 澁谷かのん
const KEKE: &str = "PL!SP-bp1-004-PR"; // 平安名すみれ
const REN: &str = "PL!SP-bp1-016-PR"; // 葉月恋(distinct name)
const SUMIRE: &str = "PL!SP-bp1-018-PR"; // 米女メイ
const WIEN: &str = "PL!SP-PR-017-PR"; // ウィーン・マルガレーテ

#[test]
fn wish_song_bp4_026_live_success_with_five_distinct_liella_revealed_gains_one_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp4-026-L");
    game.state.player1.live_card_zone.cards.push(live);

    for id in [game.id(KANON), game.id(KEKE), game.id(REN), game.id(SUMIRE), game.id(WIEN)] {
        game.state.revealed_cards.push(id);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "5 distinct Liella! members revealed -> score +1"
    );
}

#[test]
fn wish_song_bp4_026_live_success_with_four_distinct_liella_revealed_gains_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp4-026-L");
    game.state.player1.live_card_zone.cards.push(live);

    for id in [game.id(KANON), game.id(KEKE), game.id(REN), game.id(SUMIRE)] {
        game.state.revealed_cards.push(id);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "only 4 distinct Liella! members -> no score"
    );
}

/// 「『Liella!』のメンバー」 in isolation — the group clause never binds in the tests above.
///
/// Every fixture in this file is a 『Liella!』 member, so revealing **five distinct Liella!
/// members** is the only case exercised. An implementation that dropped the group filter
/// entirely — counting any five distinct revealed members — would pass both of them.
///
/// Five distinct **non**-Liella members is the isolating fixture: same count, same
/// distinctness, wrong group. Each is asserted from card data below, so a substituted printing
/// cannot quietly move a card into 『Liella!』 and invert the test.
#[test]
fn wish_song_five_distinct_non_liella_members_are_not_counted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp4-026-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Five DISTINCT prints, none of them 『Liella!』.
    let outsiders = [
        "PL!S-PR-015-PR",
        "PL!S-PR-017-PR",
        "PL!S-PR-018-PR",
        "PL!-sd1-010-SD",
        "PL!S-sd1-001-SD",
    ];
    let mut seen: Vec<String> = Vec::new();
    for no in outsiders {
        let id = game.id(no);
        game.assert_card_identity(id, no);
        let card = game.db.get_card(id).unwrap();
        let name = card.name.to_string();
        assert!(
            !seen.contains(&name),
            "precondition: the five outsiders must have five DIFFERENT names, or \
             名前が異なる is not what is being tested (saw {name:?} twice)"
        );
        seen.push(name);
        game.state.revealed_cards.push(id);
    }

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "名前が異なる『Liella!』のメンバーカードが5枚以上 — five distinct non-『Liella!』 members \
         satisfy the count and the distinctness but not the GROUP, so no score. A +1 here means \
         the 『Liella!』 filter is not being applied."
    );
}

/// 「名前が異なる」 in isolation — distinctness never binds in the tests above.
///
/// Both existing tests stage only distinct prints, so a filter that counted **all** revealed
/// cards while ignoring names entirely would pass them: 5 revealed = 5, and 4 revealed = 4.
///
/// Revealing **five** cards where two share a print makes the count 5 but the DISTINCT count 4,
/// which is the only fixture that separates them. Expect 0, because the clause is 5枚以上 of
/// cards with **different names**.
#[test]
fn wish_song_five_revealed_with_a_duplicate_name_is_only_four_distinct() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp4-026-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Five revealed cards: KANON appears twice, so five cards but four distinct names.
    for no in [KANON, KEKE, REN, SUMIRE, KANON] {
        game.state.revealed_cards.push(game.id(no));
    }

    let mut names: Vec<String> = game
        .state
        .revealed_cards
        .iter()
        .map(|&c| game.db.get_card(c).unwrap().name.to_string())
        .collect();
    let before = names.len();
    names.sort();
    names.dedup();
    assert_eq!(
        (before, names.len()),
        (5, 4),
        "precondition: five revealed cards must present exactly FOUR distinct names — the \
         fixture is what distinguishes 名前が異なる from a plain count"
    );

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "名前が異なる『Liella!』のメンバーカードが5枚以上 — five cards but only four DIFFERENT \
         names, so the clause is not met and no score. A +1 here means 名前が異なる is ignored \
         and a plain card count is used."
    );
}
