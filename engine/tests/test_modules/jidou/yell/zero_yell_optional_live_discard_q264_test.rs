use crate::helpers::*;

#[test]
fn zero_yell_q264_does_not_discard_live_for_additional_yells() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-pb2-020-R");
    let liella_live = game.id("PL!SP-sd1-023-SD");
    let fill = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = natsumi;
    game.state.player1.hand.cards.push(liella_live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(fill);
    }
    game.give_energy(5);

    // Set natsumi to wait so total_blade=0 → yell reveals 0 cards
    game.state.mods.add_orientation_modifier(natsumi, "wait");

    // Advance through LiveCardSet to performance phase
    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(liella_live);
    game.pass(); // LiveCardSetP2
    game.pass(); // FirstAttackerPerformance (LiveStart + yell + auto triggers)
    game.pass(); // SecondAttackerPerformance → LiveVictoryDetermination

    // Q264: 0 cards revealed → condition not met → ability should NOT trigger.
    // Strong form: the live card must NOT be in the discard; it stays where
    // the flow left it (hand or live zone).
    assert!(
        !game.state.player1.waitroom.cards.contains(&liella_live),
        "Q264: Liella! live card must NOT have been discarded"
    );
    assert!(
        game.state.player1.hand.cards.contains(&liella_live)
            || game
                .state
                .player1
                .live_card_zone
                .cards
                .contains(&liella_live),
        "Q264: Liella! live card should still be in hand or live zone"
    );
}
