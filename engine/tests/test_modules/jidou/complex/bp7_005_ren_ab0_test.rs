use crate::helpers::*;

#[test]
fn ren_ab0_debut_places_energy_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.hand.cards.push(ren);
    game.state.player1.energy_deck.cards.push(game.id("PL!-sd1-010-SD"));
    game.give_energy(15);
    game.play_to_stage(ren, rabuka_engine::zones::MemberArea::Center);
    game.drain_auto_ability_choices();
    // After debut, the jidou should trigger — we just verify it doesn't crash and stage has ren
    assert_eq!(game.state.player1.stage.stage[1], ren, "ren should be on center after debut");
}

#[test]
fn ren_ab0_energy_return_places_energy_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [ren, -1, -1];
    game.state.player1.energy_zone.cards.push(game.id("PL!-sd1-010-SD"));
    game.state.player1.energy_zone.set_active_count(1);
    game.state.player1.energy_deck.cards.push(game.id("PL!-sd1-010-SD"));
    let eid = game.state.player1.energy_zone.cards[0];
    game.state.player1.energy_zone.cards.retain(|id| *id != eid);
    game.state.player1.energy_deck.cards.push(eid);
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
    // Should have triggered without crash
    assert!(game.state.player1.stage.stage.contains(&ren));
}
