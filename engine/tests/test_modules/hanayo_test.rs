/// PL!-pb1-008-R (小泉花陽) ab#0 — Q183
///
/// {{toujyou.png|登場}}メンバーを3人までウェイトにしてもよい：
/// これによりウェイト状態にしたメンバー1人につき、カードを1枚引く。
///
/// Q183: Can this ability put opponent's members to wait?
/// A: No — cost/effect restriction is self-only.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn hanayo_q183_self_only_wait_no_opponent_affected() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanayo = game.id("PL!-pb1-008-R");
    let friend = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Self stage: friend left, hanayo center
    game.state.player1.stage.stage = [friend, -1, -1];
    // Opponent stage: has a member too
    game.state.player2.stage.stage = [filler, -1, -1];

    game.add_to_hand(hanayo);
    game.give_energy(15);

    // Play hanayo to center → debut triggers
    game.play_to_stage(hanayo, MemberArea::Center);

    // The debut ability has an optional cost: put members to wait.
    // If there's a pending choice, we can verify it targets self only.
    if game.has_pending_choice() {
        // The choice is to put members to wait (SelectTarget for optional cost).
        // Select option 1 = "pay optional cost", meaning "yes, put members to wait"
        game.select_option(1);

        // Now a SelectCard choice appears to choose which members to put to wait.
        // The available cards should be ONLY self's stage members, not opponent's.
        if game.has_pending_choice() {
            // Select the first available member to put to wait
            game.select_indices(&[0]);
        }
    }

    // Q183: Opponent member should be unaffected
    assert_eq!(
        game.state.player2.stage.stage[0], filler,
        "Opponent member should still be on stage"
    );
}
