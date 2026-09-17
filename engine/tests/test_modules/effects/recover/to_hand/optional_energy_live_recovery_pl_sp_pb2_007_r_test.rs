use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_sp_pb2_007_r_paid_three_energy_recovers_liella_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-pb2-007-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    let live = game.new_id("PL!SP-bp1-026-L");
    game.state.player1.waitroom.cards.push(live);

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    game.select_option(1);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "paid 3 -> 『Liella!』 live retrieved to hand"
    );
}

#[test]
fn pl_sp_pb2_007_r_unpayable_energy_keeps_live_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-pb2-007-R");
    game.state.player1.stage.stage[1] = me;
    let live = game.new_id("PL!SP-bp1-026-L");
    game.state.player1.waitroom.cards.push(live);

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.has_pending_choice(),
        "unpayable optional {{E}}{{E}}{{E}} gate must auto-skip without prompting"
    );

    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "declined -> live stays in the waitroom"
    );
}
