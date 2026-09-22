use crate::helpers::*;
use rabuka_engine::ability::resolver::AbilityResolver;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Energy-zone selection must respect active-vs-waited state:
// - Under-member menu shows both counts and lets either be picked
//   (both are legal placements).
// - Removing waited cards must NOT move the active counter (positional
//   convention: indices [0..active) are active).
// - Tapping an already-waited card to wait is a no-op for the counter.
// ====================================================================

/// Mixed pile read live by the debut menu: 20 energy with 16 active, Kasumi's
/// play cost takes 13 → menu sees 3 active + 17 waited (count 2). The mix is
/// set BEFORE playing because menu text is rendered at prompt time.
fn mixed_zone_kasumi(game: &mut TestGame) {
    let kasumi = game.id("PL!N-pb1-002-R");
    game.state.player1.hand.cards.push(kasumi);
    game.give_energy(20);
    game.state.player1.energy_zone.set_active_count(16);
    game.play_to_stage(kasumi, MemberArea::Center);
    assert!(
        game.has_pending_choice(),
        "under-member energy menu prompted"
    );
}

#[test]
fn under_member_menu_shows_active_waited_counts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    mixed_zone_kasumi(&mut game);

    let desc = game.get_pending_choice().description().to_string();
    assert!(
        desc.contains("(active: 3, waited: 17)"),
        "menu states the pile composition, got: {}",
        desc
    );
}

#[test]
fn under_member_remove_waited_keeps_active_count() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    mixed_zone_kasumi(&mut game);

    // Indices 18,19 are waited (active prefix is 0..3).
    game.select_indices(&[18, 19]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "removing waited cards must not move the active counter"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        18,
        "2 energy left the zone"
    );
    assert_eq!(
        game.state
            .player1
            .stage
            .get_under_cards(MemberArea::Center)
            .len(),
        2,
        "2 energy placed under the member"
    );
}

#[test]
fn under_member_remove_active_decrements_counter() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    mixed_zone_kasumi(&mut game);

    game.select_indices(&[0, 1]);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "removing 2 active cards drops the counter 3 -> 1"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        18,
        "2 energy left the zone"
    );
}

#[test]
fn tap_already_waited_energy_leaves_counter() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let e1 = game.new_id("LL-E-001-SD");
    let e2 = game.new_id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e1);
    game.state.player1.energy_zone.cards.push(e2);
    game.state.player1.energy_zone.set_active_count(1);

    let mut resolver = AbilityResolver::new(game.state.card_database.clone(), None);
    resolver
        .execute_selected_energy_zone_cards(&mut game.state, &[1], 1)
        .expect("tap waited executes");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        1,
        "tapping an already-waited card must not decrement"
    );

    resolver
        .execute_selected_energy_zone_cards(&mut game.state, &[0], 1)
        .expect("tap active executes");
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "tapping the active card decrements"
    );
}
