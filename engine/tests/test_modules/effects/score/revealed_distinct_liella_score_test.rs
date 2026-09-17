use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const KANON: &str = "PL!SP-pb1-001-PR"; // 澁谷かのん
const KEKE: &str = "PL!SP-bp1-004-PR"; // 唐可可
const REN: &str = "PL!SP-bp1-016-PR"; // 葉月 凛? (distinct name)
const SUMIRE: &str = "PL!SP-bp1-018-PR"; // 平安名すみれ
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
