use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

// ====================================================================
// PL!SP-bp1-009-R 鬼塚夏美 (thin-coverage: 1 test fn before this)
// {{起動}}{{ターン1回}}{{E}}：カードを1枚引き、手札を1枚控え室に置く。
//
// Turn-1 activation, 1-energy cost: draw 1, then discard 1 from hand.
// Pins the FULL printed behavior through the public pipeline: the offer,
// the energy payment, draw+discard, use-limit recording, and no re-offer.
// ====================================================================

fn count_natsumi_use_offers(game: &TestGame, natsumi: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(natsumi)
        })
        .count()
}

fn activate_resolving_choices(game: &mut TestGame, card_id: i16) {
    game.activate_ability_for(Side::P1, card_id)
        .expect("activation failed");
    let mut steps = 0;
    while game.has_pending_choice() && steps < 15 {
        steps += 1;
        answer_choice(game, 0);
    }
    assert!(
        !game.has_pending_choice(),
        "prompts should terminate after answering"
    );
    game_setup::settle_single_player_state(&mut game.state);
}

#[test]
fn natsumi_turn1_energy_draw_discard_full_behavior() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp1-009-R");
    assert_eq!(
        game.db.get_card(natsumi).unwrap().card_no,
        "PL!SP-bp1-009-R",
        "staged the R print of Natsumi"
    );
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    game.state.player1.stage.stage[1] = natsumi;
    game.give_energy(2);
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));
    game.add_to_hand(game.new_id("PL!-sd1-010-SD"));

    let active_before = game.state.player1.energy_zone.active_count();
    let hand_before = game.state.player1.hand.cards.len();
    let wait_before = game.state.player1.waitroom.cards.len();

    assert_eq!(
        count_natsumi_use_offers(&game, natsumi),
        1,
        "fresh turn offers the ability exactly once"
    );
    activate_resolving_choices(&mut game, natsumi);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before - 1,
        "1E cost was paid"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "drew 1 then discarded 1 (net unchanged)"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        wait_before + 1,
        "the discarded hand card reached the waitroom"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(natsumi, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
    assert_eq!(
        count_natsumi_use_offers(&game, natsumi),
        0,
        "consumed ターン1回 ability must not be re-offered"
    );
}
