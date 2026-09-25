/// Edge case tests for continuous (常時) abilities.
///
/// Tests complex compound conditions, cross-player conditions,
/// distinct-name checks, and conditional score modifications.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Target: PL!S-bp2-001-R (Riko)
// 常時: If own success_live_card_zone has 0 cards AND opponent has 1+,
//       gain 3 blade.
// So: compound(AND) condition with own zone count and opponent zone count.
// ====================================================================

/// Edge case: Condition fully met → gain 3 blade.
#[test]
fn own_success_empty_opponent_nonempty_grants_three_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    let opponent_live = game.id("PL!-sd1-019-SD");

    // Put riko on center stage
    game.add_to_stage(MemberArea::Center, riko);

    // Opponent has 1+ success live card
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opponent_live);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 3,
        "Riko: own=0, opponent>=1 → should gain 3 blade, got {}",
        blade_mod
    );
}

/// Edge case: Condition NOT met (own has cards) → gain 0.
#[test]
fn own_success_nonempty_opponent_empty_grants_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    let live_card = game.id("PL!-sd1-019-SD");

    game.add_to_stage(MemberArea::Center, riko);

    // Own success zone has a card (violates condition)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(live_card);

    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 0,
        "Riko: own has card → condition not met, blade should be 0, got {}",
        blade_mod
    );
}

/// Edge case: Condition PARTIALLY met (own=0, opponent=0) → gain 0.
#[test]
fn both_success_zones_empty_grant_no_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    game.add_to_stage(MemberArea::Center, riko);

    // Neither player has success cards
    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 0,
        "Riko: both empty → condition not met, blade should be 0, got {}",
        blade_mod
    );
}

/// Edge case: Condition goes from met → unmet when opponent's cards are removed.
#[test]
fn opponent_success_removal_clears_three_blade_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let riko = game.id("PL!S-bp2-001-R");
    let opponent_live = game.id("PL!-sd1-019-SD");

    game.add_to_stage(MemberArea::Center, riko);
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opponent_live);

    // Condition met
    game.state.recalculate_constants();
    assert_eq!(game.state.mods.get_blade_modifier(riko), 3);

    // Remove opponent card → condition no longer met
    game.state.player2.success_live_card_zone.cards.clear();
    game.state.recalculate_constants();

    let blade_mod = game.state.mods.get_blade_modifier(riko);
    assert_eq!(
        blade_mod, 0,
        "Riko: opponent card removed → blade should be 0, got {}",
        blade_mod
    );
}
