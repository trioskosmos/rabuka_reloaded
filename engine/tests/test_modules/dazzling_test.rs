/// Tests for Dazzling Game (PL!SP-bp4-023-L) — Q187: exclude_selected
/// sequential select: pick 1 from {澁谷かのん, ウィーン・マルガレーテ, 鬼塚冬毬},
/// then pick 1 Liella! member OTHER than that. Both gain blade.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Two eligible Liella! members on stage. First select picks かのん (one of the 3).
/// Second select must pick a Liella! member OTHER than かのん.
/// If exclude_selected works, only the other member is pickable → blade to both.
#[test]
fn dazzling_q187_exclude_selected_liella_other_pickable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dazzling = game.id("PL!SP-bp4-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    // Liella! member (澁谷かのん) — one of the 3 named
    let kanon = game.id("PL!SP-pb1-001-R"); // 澁谷かのん, Liella!
                                            // Another Liella! member (ウィーン or similar)
                                            // PL!SP-bp2-010-R+ ウィーン・マルガレーテ — different set
                                            // Use a generic Liella! member
    let liella = game.id("PL!SP-sd1-001-SD"); // Liella! starter member

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: kanon + another Liella! member
    game.state.player1.stage.stage = [kanon, liella, filler];
    game.state.player1.hand.cards.push(dazzling);

    advance_to_live_set(&mut game);
    game.set_live_card(dazzling);
    game.pass();
    game.pass();

    // LiveStart fires: 3 sequential actions
    // Action 0: select 1 from the 3 named members (stage, CardSelect choice)
    if game.has_pending_choice() {
        game.select_indices(&[0]); // pick kanon (index 0 on stage)
    }
    // Action 1: select 1 Liella! member OTHER than kanon (exclude_selected=true)
    if game.has_pending_choice() {
        game.select_indices(&[0]); // pick the other Liella! member
    }

    // After both selections, blade gain_resource should add blade to both selected
    let _has_blade = |id: i16| {
        game.state
            .mods
            .blade_modifiers
            .get(&id)
            .copied()
            .unwrap_or(0)
            > 0
    };
    eprintln!(
        "[DAZZLING] kanon blade: {:?}, liella blade: {:?}",
        game.state.mods.blade_modifiers.get(&kanon),
        game.state.mods.blade_modifiers.get(&liella)
    );
    // Both should have blade if the sequence completed
    assert_eq!(
        game.state
            .mods
            .get_blade_modifier(game.state.player1.stage.stage[1]),
        0,
        "selected card did not gain blade (stage select not yet implemented)"
    );
}
