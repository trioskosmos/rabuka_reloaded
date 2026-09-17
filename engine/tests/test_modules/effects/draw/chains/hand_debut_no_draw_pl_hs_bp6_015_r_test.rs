use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_hs_bp6_015_r_hand_debut_leaves_hand_count_one_lower() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!HS-bp6-015-R");
    game.add_to_hand(me);
    game.give_energy(5);
    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(me, MemberArea::Center);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "normal hand debut -> no draw bonus"
    );
}
