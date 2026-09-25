use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn chisato_pb1_003_rotates_both_players_while_only_player_one_is_syncrie() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = game.id("PL!SP-pb1-003-R");
    let syncrie_a = game.id("PL!SP-pb1-014-N");
    let syncrie_b = game.id("PL!SP-pb1-019-N");
    let opp_left = game.id("PL!-sd1-010-SD");
    let opp_center = game.id("PL!-sd1-013-SD");
    let opp_right = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [syncrie_a, syncrie_b, -1];
    game.state.player2.stage.stage = [opp_left, opp_center, opp_right];
    game.state.player1.hand.cards.push(chisato);
    game.give_energy(9);
    game.play_to_stage(chisato, MemberArea::RightSide);

    assert_eq!(game.state.player1.stage.stage, [syncrie_b, chisato, syncrie_a]);
    assert_eq!(game.state.player2.stage.stage, [opp_center, opp_right, opp_left]);
    assert!(!game.has_pending_choice());
}
