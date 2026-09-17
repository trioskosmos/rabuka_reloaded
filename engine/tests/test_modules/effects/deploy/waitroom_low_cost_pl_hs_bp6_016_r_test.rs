use crate::helpers::*;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_hs_bp6_016_r_activation_moves_low_cost_hasunosora_member_from_waitroom_to_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!HS-bp6-016-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(6);
    let kamaru = game.new_id("PL!HS-bp2-004-R");
    game.state.player1.waitroom.cards.push(kamaru);
    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.stage.stage.contains(&kamaru),
        "cost<=4 『蓮ノ空』 member debuted into an empty area"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&kamaru),
        "deployed member left the waitroom"
    );
}
