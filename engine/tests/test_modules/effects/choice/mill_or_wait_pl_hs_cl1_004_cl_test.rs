use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn pl_hs_cl1_004_cl_debut(game: &mut TestGame) -> i16 {
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    let me = game.new_id("PL!HS-cl1-004-CL");
    game.add_to_hand(me);
    game.give_energy(20);
    game.play_to_stage(me, MemberArea::LeftSide);
    me
}

#[test]
fn pl_hs_cl1_004_cl_mill_option_removes_three_deck_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _me = pl_hs_cl1_004_cl_debut(&mut game);
    let deck_before = game.state.player1.main_deck.cards.len();
    game.select_option(0);
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 3,
        "chose mill -> deck top 3 moved to waitroom"
    );
}

#[test]
fn pl_hs_cl1_004_cl_wait_option_waits_cost_two_opponent_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = pl_hs_cl1_004_cl_debut(&mut game);
    let _ = me;
    let enemy = game.id("PL!SP-PR-010-PR");
    game.state.player2.stage.stage[1] = enemy;
    game.select_option(1);
    assert_eq!(
        game.state.mods.get_orientation_modifier(enemy),
        Some("wait"),
        "chose wait -> enemy cost<=2 member waited"
    );
}
