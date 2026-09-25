use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const KANON: &str = "PL!SP-pb1-001-PR"; // 澁谷かのん
const KEKE: &str = "PL!SP-bp1-004-PR"; // 唐可可
const CHISATO: &str = "PL!SP-pb1-014-PR"; // 嶋野あい? distinct Liella name

#[test]
fn sp_bp4_006_live_success_with_three_distinct_liella_revealed_retrieves_liella_live_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp4-006-R");
    game.state.player1.stage.stage[0] = me;

    for id in [game.id(KANON), game.id(KEKE), game.id(CHISATO)] {
        game.state.revealed_cards.push(id);
    }
    let liella_live = game.id("PL!SP-bp1-023-L"); // Liella! live card
    game.state.revealed_cards.push(liella_live);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Liella! live card retrieved from revealed cards to hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&liella_live),
        "the retrieved card is the revealed Liella! live card"
    );
}
