use crate::helpers::*;

const FILLER: &str = "PL!N-sd1-010-SD";

fn answer_all(game: &mut TestGame, idx: usize) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        if game.pending_choice_type().as_deref() == Some("SelectHeartColor") {
            game.select_choice_option(idx);
        } else {
            game.select_indices(&[idx]);
        }
    }
}

#[test]
fn pl_pr_017_pr_activation_recovers_mus_live_and_activates_two_energy_at_nine_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-PR-017-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(2);
    let mus_live = game.new_id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(mus_live);
    let s9 = game.new_id("PL!S-pb1-023-L");
    game.state.player1.success_live_card_zone.cards.push(s9);

    game.activate_ability(me);
    answer_all(&mut game, 0);

    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "μ's live recovered to hand"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4,
        "score total 9 >= 9 -> 2 energies ACTIVATED (2->4)"
    );
}

#[test]
fn pl_pr_017_pr_activation_empty_success_zone_recovers_live_without_energy_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-PR-017-PR");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(2);
    let mus_live = game.new_id("PL!-sd1-019-SD");
    game.state.player1.waitroom.cards.push(mus_live);

    game.activate_ability(me);
    answer_all(&mut game, 0);

    assert!(game.state.player1.hand.cards.contains(&mus_live));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "no success cards -> no activation"
    );
}
