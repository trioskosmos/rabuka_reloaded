use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// 元気全開DAY！DAY！DAY！ invalidates its own live_success when heart02 ≥ 6.
/// Aqours members on stage with enough heart02 → condition met → live_success negated.
#[test]
fn genki_zenkai_invalidates_own_live_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let genki = game.id("PL!S-pb1-019-L"); // live card
    let chika = game.id("PL!S-sd1-010-SD"); // Aqours, heart02:2
    let ruby = game.id("PL!S-pb1-018-N"); // Aqours, heart02:2
    let yoshiko = game.id("PL!S-bp6-015-N"); // Aqours, heart02:2
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 3 Aqours members with total heart02 = 6 → condition met (≥ 6)
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::Center, ruby);
    game.add_to_stage(MemberArea::RightSide, yoshiko);

    // Main decks
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(genki);
    game.set_live_card(genki);

    // Advance to live_start (ab#0 fires: condition check → invalidate live_success)
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // After live_start resolved, the card should be in negated_abilities
    assert!(
        game.state.negated_abilities.contains(&genki),
        "Genki Zenkai should be in negated_abilities after live_start triggers invalidation"
    );

    // Advance through performance → live_success should NOT fire
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Live_success was invalidated — opponent's energy deck/zone unaffected
    assert_eq!(
        game.state.player2.energy_zone.cards.len(),
        0,
        "Genki Zenkai invalidation: opponent energy zone should be empty"
    );
}

/// Control: Genki Zenkai with heart02 < 6 → live_success fires → opponent places energy.
#[test]
fn genki_zenkai_live_success_fires_when_condition_not_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let genki = game.id("PL!S-pb1-019-L");
    let chika = game.id("PL!S-sd1-010-SD"); // Aqours, heart02:2
    let ruby = game.id("PL!S-pb1-018-N"); // Aqours, heart02:2
    let filler = game.id("PL!-sd1-010-SD"); // NOT Aqours → not counted for condition

    // Stage: 2 Aqours (total heart02 = 4) + 1 non-Aqours heart02 = 4 < 6 → condition NOT met
    game.add_to_stage(MemberArea::LeftSide, chika);
    game.add_to_stage(MemberArea::Center, ruby);
    game.add_to_stage(MemberArea::RightSide, filler);

    // Opponent energy deck — need real energy card for the move-cards effect
    let energy_card = game.id("LL-E-001-SD");
    game.state.player2.energy_deck.cards.clear();
    for _ in 0..10 {
        game.state.player2.energy_deck.cards.push(energy_card);
    }

    // Main decks
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.state.player1.hand.cards.push(genki);
    game.set_live_card(genki);

    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Condition NOT met → card should NOT be in negated_abilities
    assert!(
        !game.state.negated_abilities.contains(&genki),
        "Genki Zenkai should NOT be in negated_abilities (condition not met)"
    );

    // Advance through performance → live_success SHOULD fire
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Live_success fired — opponent should have 1 wait-energy in energy zone
    let p2_energy_zone_count = game.state.player2.energy_zone.cards.len();
    assert!(
        p2_energy_zone_count >= 1,
        "Genki Zenkai live_success should fire: opponent energy zone should have ≥1 card (got {})",
        p2_energy_zone_count
    );
}
