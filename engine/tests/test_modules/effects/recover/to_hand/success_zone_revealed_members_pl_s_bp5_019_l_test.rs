use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_s_bp5_019_l_two_own_success_cards_retrieve_two_revealed_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let live = game.id("PL!S-bp5-019-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    for no in ["PL!-sd1-019-SD", "PL!HS-bp2-020-L"] {
        let s = game.new_id(no);
        game.state.player1.success_live_card_zone.cards.push(s);
    }
    let m1 = game.new_id("PL!N-sd1-006-P");
    let m2 = game.new_id("PL!N-bp1-009-R");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m1);
    game.state.revealed_cards.push(m2);
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        game.has_pending_choice(),
        "up-to-2 member selection prompt expected"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (revealed_cards, count=2)"
    );
    game.select_indices(&[0, 1]);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.hand.cards.contains(&m1) && game.state.player1.hand.cards.contains(&m2),
        "gate met -> up to 2 members retrieved to hand"
    );
}
