use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_sp_pr_018_pr_seven_liella_reveals_add_one_energy_on_live_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!SP-PR-018-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 3);
    let zone_before = game.state.player1.energy_zone.cards.len();
    game.state.revealed_cards.clear();
    for _ in 0..7 {
        game.state
            .revealed_cards
            .push(game.new_id("PL!SP-bp1-026-L"));
    }
    game.state.revealed_cards.push(filler);
    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        ">=7 Liella! reveals -> energy placed from the deck"
    );
}

#[test]
fn pl_sp_pr_018_pr_six_liella_reveals_add_no_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let me = game.id("PL!SP-PR-018-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 3);
    let zone_before = game.state.player1.energy_zone.cards.len();
    game.state.revealed_cards.clear();
    for _ in 0..6 {
        game.state
            .revealed_cards
            .push(game.new_id("PL!SP-bp1-026-L"));
    }
    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert_eq!(game.state.player1.energy_zone.cards.len(), zone_before);
}
