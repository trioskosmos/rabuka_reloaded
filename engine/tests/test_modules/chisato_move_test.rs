/// PL!SP-bp2-003-R (嵐千砂都) Q126
///
/// {{jidou.png|自動}}{{turn1.png|ターン1回}}このメンバーがエリアを移動したとき、
/// 自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
///
/// Q126: Does moving from stage→waitroom (zone change) trigger this?
/// A: No — only area-to-area movement (position change) on stage.
use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

fn setup_energy_deck(game: &mut TestGame) {
    let energy = game.id("LL-E-001-SD");
    for _ in 0..10 {
        game.state.player1.energy_deck.cards.push(energy);
    }
}

#[test]
fn chisato_q126_area_move_triggers_energy_placement() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let chisato = game.id("PL!SP-bp2-003-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: chisato on left, filler on right (swap to trigger area move)
    game.state.player1.stage.stage = [chisato, -1, filler];
    setup_energy_deck(&mut game);

    let energy_before = game.state.player1.energy_zone.cards.len();
    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    // Perform position change via the stage API
    game.state
        .player1
        .stage
        .position_change(MemberArea::LeftSide, MemberArea::RightSide)
        .expect("Position change should succeed");

    // Manually trigger auto abilities for the player
    let player_id = game.state.player1.id.clone();
    TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &player_id);
    game.state.process_pending_auto_abilities(&player_id);

    // Q126: area move should trigger the auto ability,
    // placing 1 energy card from energy deck → energy zone
    let energy_after = game.state.player1.energy_zone.cards.len();
    assert_eq!(
        energy_after,
        energy_before + 1,
        "Area move should trigger 1 energy placement: {} → {}",
        energy_before,
        energy_after
    );

    let energy_deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(
        energy_deck_after,
        energy_deck_before - 1,
        "Energy deck should lose 1 card: {} → {}",
        energy_deck_before,
        energy_deck_after
    );
}
