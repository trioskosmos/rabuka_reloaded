use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn note_mermaid_two_distinct_kaleidoscore_members_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-pb1-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Two different KALEIDOSCORE characters.
    let ren = game.id("PL!SP-bp1-013-PR");
    let wien = game.id("PL!SP-PR-017-PR");
    game.state.player1.stage.stage[0] = ren;
    game.state.player1.stage.stage[1] = wien;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "two distinct KALEIDOSCORE members -> score +1"
    );
}

#[test]
fn note_mermaid_duplicate_kaleidoscore_names_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-pb1-024-L");
    game.state.player1.live_card_zone.cards.push(live);

    // Two copies of the SAME character -> not "名前の異なる".
    let ren1 = game.id("PL!SP-bp1-013-PR");
    let ren2 = game.new_id("PL!SP-pb1-013-PR");
    game.state.player1.stage.stage[0] = ren1;
    game.state.player1.stage.stage[1] = ren2;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "duplicate names do not count as distinct"
    );
}
