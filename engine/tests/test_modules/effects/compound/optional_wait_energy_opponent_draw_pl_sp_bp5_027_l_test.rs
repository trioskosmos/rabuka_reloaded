use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

fn hot_passion_pl_sp_bp5_027_l_setup(game: &mut TestGame) -> i16 {
    let live = game.id("PL!SP-bp5-027-L");
    fill_decks(game, {
        let f = game.new_id("PL!-sd1-010-SD");
        f
    });
    game.state.player1.live_card_zone.cards.push(live);
    fill_energy_deck(game, 0, 2);
    live
}

#[test]
fn hot_passion_pl_sp_bp5_027_l_accept_waited_energy_opponent_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = hot_passion_pl_sp_bp5_027_l_setup(&mut game);

    let p2_hand_before = game.state.player2.hand.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        game.has_pending_choice(),
        "optional energy placement prompted"
    );
    game.select_option(1);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        1,
        "one energy card placed into the energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "the placed energy arrives WAITED"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before + 1,
        "placement accepted -> opponent draws 1"
    );
}

#[test]
fn hot_passion_pl_sp_bp5_027_l_decline_no_energy_or_opponent_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = hot_passion_pl_sp_bp5_027_l_setup(&mut game);

    let p2_hand_before = game.state.player2.hand.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        game.has_pending_choice(),
        "optional energy placement prompted"
    );
    game.select_indices(&[]);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        0,
        "declined -> no energy placed"
    );
    assert_eq!(
        game.state.player2.hand.cards.len(),
        p2_hand_before,
        "declined -> opponent draws nothing"
    );
}
