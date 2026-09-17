use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

fn fire_debut_accept(game: &mut TestGame, me: i16) {
    fire_trigger(game, me, AbilityTrigger::Debut, "登場");
    assert!(
        game.has_pending_choice(),
        "optional discard cost prompt expected on debut"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard (hand cost gate)"
    );
    game.select_option(0);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
}

#[test]
fn pl_bp5_014_n_debut_look_four_takes_member_with_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!-bp5-014-N");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let eli = game.new_id("PL!-PR-002-PR");
    game.state.player1.main_deck.cards.insert(0, eli);

    fire_debut_accept(&mut game, me);

    assert!(
        game.state.player1.hand.cards.contains(&eli),
        "member holding heart06 matches the OR-color filter"
    );
}
