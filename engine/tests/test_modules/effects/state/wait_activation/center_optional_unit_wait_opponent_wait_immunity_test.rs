use crate::helpers::*;
use crate::test_modules::support::bp7_wait_immunity_helpers::*;

#[test]
fn center_optional_unit_wait_opponent_wait_is_blocked_by_immunity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Player2 protects their 果南 (Aqours, active).
    let p2_kanan = p2_establish_wait_immunity(&mut game);

    // Player1 plays 真姫 at center → ab#0 makes the opponent (player2) wait one of
    // their own active members.
    let maki = game.id("PL!-pb1-015-R");
    let bibi = game.id("PL!-sd1-011-SD"); // BiBi member for the condition
    game.state.player1.hand.cards.push(maki);
    game.state.player1.stage.stage[0] = bibi;
    game.state.player1.stage.stage[1] = -1;
    game.give_energy(11);
    game.play_to_stage(maki, rabuka_engine::zones::MemberArea::Center);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        // Pay the optional cost (wait a BiBi member), then the opponent's own-wait.
        if game.pending_choice_type().as_deref() == Some("SelectCard") {
            game.select_indices(&[0]);
        } else {
            game.select_choice_option(1);
        }
    }

    assert!(
        !is_waited(&game, p2_kanan),
        "真姫's ab#0 opponent-wait must be blocked by wait-immunity"
    );
}
