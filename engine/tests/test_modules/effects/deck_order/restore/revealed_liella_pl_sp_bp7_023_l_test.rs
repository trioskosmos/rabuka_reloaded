use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_sp_bp7_023_l_selected_revealed_liella_card_increases_deck_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let live = game.id("PL!SP-bp7-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    let liella_card = game.new_id("PL!SP-bp1-026-L");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(liella_card);
    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        assert!(
            game.pending_choice_type().is_some(),
            "prompt {guard} must carry a choice identity"
        );
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before + 1,
        "revealed Liella! card placed onto the deck top"
    );
}
