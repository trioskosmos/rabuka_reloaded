use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn pl_n_pb1_006_r_wait_self_activates_one_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(10);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let energy_before = game.state.player1.energy_zone.active_count();
    game.activate_ability(card);
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some("wait"),
        "Cost: card must be in wait state after activation"
    );
    let energy_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        energy_after,
        energy_before + 1,
        "Effect: should activate exactly 1 energy (was {}, now {})",
        energy_before,
        energy_after
    );
}

#[test]
fn pl_n_pb1_006_r_activates_wait_energy_without_changing_card_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.state.player1.energy_zone.set_active_count(0);
    let active_before = game.state.player1.energy_zone.active_count();
    let total_before = game.state.player1.energy_zone.cards.len();
    game.activate_ability(card);
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some("wait"),
        "Cost paid: card must be wait"
    );
    let active_after = game.state.player1.energy_zone.active_count();
    assert_eq!(
        active_after,
        active_before + 1,
        "Effect: should activate exactly 1 energy (was {} active, now {})",
        active_before,
        active_after
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        total_before,
        "Energy card count unchanged"
    );
}

#[test]
fn pl_n_pb1_006_r_reactivated_member_can_activate_energy_three_times() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    let energy_before = game.state.player1.energy_zone.active_count();
    for i in 0..3 {
        game.activate_ability(card);
        assert_eq!(
            game.state.player1.energy_zone.active_count(),
            energy_before + i + 1,
            "Activation {} should give 1 energy",
            i + 1
        );
        if i < 2 {
            game.state.mods.add_orientation_modifier(card, "active");
        }
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some("wait"),
        "Card should be wait after multiple activations"
    );
}
