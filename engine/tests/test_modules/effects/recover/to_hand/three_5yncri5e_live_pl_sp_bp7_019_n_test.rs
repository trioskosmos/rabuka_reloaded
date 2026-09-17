use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_sp_bp7_019_n_three_5yncri5e_members_recover_waitroom_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!SP-bp7-019-N");
    game.add_to_hand(me);
    game.give_energy(30);
    let s1 = game.id("PL!SP-PR-005-PR");
    let s2 = game.id("PL!SP-PR-008-PR");
    game.state.player1.stage.stage[0] = s1;
    game.state.player1.stage.stage[1] = s2;
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    game.play_to_stage(me, MemberArea::RightSide);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "3x 5yncri5e! staged -> live card retrieved to hand"
    );
}

#[test]
fn pl_sp_bp7_019_n_only_two_5yncri5e_members_do_not_recover_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    let me = game.id("PL!SP-bp7-019-N");
    game.add_to_hand(me);
    game.give_energy(30);
    let s1 = game.id("PL!SP-PR-005-PR");
    game.state.player1.stage.stage[0] = s1;
    let outsider = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = outsider;
    let mus_live = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    game.play_to_stage(me, MemberArea::RightSide);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        !game.state.player1.hand.cards.contains(&mus_live),
        "only 2x 5yncri5e! -> no retrieval"
    );
}
