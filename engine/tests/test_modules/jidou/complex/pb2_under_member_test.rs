use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pb2_kinako_under_member_both_players() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id("PL!-sd1-010-SD");

    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    for _ in 0..15 {
        let e = game.id("LL-E-001-SD");
        game.state.player2.energy_zone.cards.push(e);
    }
    game.state.player2.energy_zone.set_active_count(15);

    // P1
    let p1_kinako = game.id("PL!SP-pb2-006-R");
    let p1_chisato = game.id("PL!SP-pb2-025-N");
    let p1_liella = game.id("PL!SP-pb1-006-R");

    game.state.player1.stage.stage[0] = p1_kinako;
    game.state.player1.waitroom.cards.push(p1_liella);
    game.state.player1.hand.cards.push(p1_chisato);
    game.play_to_stage(p1_chisato, MemberArea::Center);

    assert!(game.has_pending_choice());
    let acts = game.generated_actions();
    let left_idx = acts
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left"))
        .expect("left destination");
    game.select_generated(left_idx);

    let under_center = game.state.player1.stage.under_cards[1].to_vec();
    let under_left = game.state.player1.stage.under_cards[0].to_vec();
    assert!(
        under_center.contains(&p1_liella) || under_left.contains(&p1_liella),
        "P1: Liella! under きな子. center={:?} left={:?}",
        under_center,
        under_left
    );

    // P2's turn
    let p2_kinako = game.id("PL!SP-pb2-006-R");
    let p2_chisato = game.id("PL!SP-pb2-025-N");
    let p2_liella = game.id("PL!SP-pb1-006-R");

    game.state.player2.stage.stage[0] = p2_kinako;
    game.state.player2.waitroom.cards.push(p2_liella);
    game.state.player2.hand.cards.push(p2_chisato);

    game.pass();
    game.pass();
    game.pass();
    game.pass();

    rabuka_engine::turn::TurnEngine::execute_main_phase_action(
        &mut game.state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(p2_chisato),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("P2 play");

    assert!(game.has_pending_choice());
    let acts = game.generated_actions();
    let left_idx = acts
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left"))
        .expect("P2 left destination");
    game.select_generated(left_idx);

    let under_center_p2 = game.state.player2.stage.under_cards[1].to_vec();
    let under_left_p2 = game.state.player2.stage.under_cards[0].to_vec();
    assert!(
        under_center_p2.contains(&p2_liella) || under_left_p2.contains(&p2_liella),
        "P2: Liella! under きな子. center={:?} left={:?}",
        under_center_p2,
        under_left_p2
    );
}
