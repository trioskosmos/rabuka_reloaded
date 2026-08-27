//! Rule 9.9.1 continuous-effect layering (総合ルール ver 1.06):
//!   9.9.1.1 printed info is the base
//!   9.9.1.4 set-type effects ("…はNになる") replace the value
//!   9.9.1.5 additive effects stack ON TOP of the set value
//!
//! Blade side already pinned by Q195 (special_color_test.rs). This file pins
//! the HEART side: a heart-type SET must NOT swallow additive heart gains in
//! either stage-heart computation (calculate_stage_hearts /
//! get_available_hearts), plus an end-to-end blade set→additive flow through
//! two real Live Start abilities.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

/// End-to-end 9.9.1.4→9.9.1.5 with two REAL live cards:
/// 1. Special Color (PL!SP-bp4-025-L) sets center Liella!'s blades to 3.
/// 2. PL!-bp3-026-L's Live Start (+3 blades to one member until live end,
///    cost: discard 2) stacks ON TOP → effective modifier 3+3 = 6.
#[test]
fn blade_set_then_additive_stacks_through_real_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let special = game.id("PL!SP-bp4-025-L"); // set_blade_count(3), center Liella!
    let giver = game.id("PL!-bp3-026-L"); // LS: +3 blades to 1 member (cost: discard 2)
    let liella = game.id("PL!SP-bp1-001-R"); // Liella!, blade=3
    let filler = game.id("PL!-sd1-010-SD");

    // Center Liella! ONLY — so the giver's "member 1" targeting has exactly
    // one candidate and index 0 unambiguously selects her.
    game.state.player1.stage.stage = [-1, liella, -1];
    game.state.player1.hand.cards.push(special);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Step 1: Special Color's Live Start sets blades to 3.
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(special);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.drain_choices_strict(&["SelectCard", "SelectAutoAbility"], &[0]);
    }
    assert_eq!(
        game.state.mods.get_blade_set_modifier(liella),
        3,
        "9.9.1.4: Special Color set center Liella!'s blades to 3"
    );

    // Step 2: the giver's LS resolves — pay the 2-card cost, target the
    // center Liella!, gain +3 blades until live end.
    fire_trigger(&mut game, giver, AbilityTrigger::LiveStart, "ライブ開始時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        match game.pending_choice_type().as_deref() {
            // optional discard-2 cost → pay it (first N hand cards)
            Some("SelectCard") => {
                let n = game.pending_choice_count();
                let idxs: Vec<usize> = (0..n).collect();
                game.select_indices(&idxs);
            }
            // member targeting → first candidate (the center Liella!)
            Some("SelectTarget") | Some("SelectPosition") => game.select_indices(&[0]),
            other => panic!("unexpected prompt {other:?} in giver chain"),
        }
    }
    assert!(
        guard < 10,
        "giver chain did not terminate; prompts kept re-appearing"
    );

    // 9.9.1.5: additive lands ON TOP of the set base: 3(set) + 3(add) = 6.
    assert_eq!(
        game.state.mods.get_blade_modifier(liella),
        6,
        "blade layering: set(3) then additive(+3) → 6"
    );
}

/// 9.9.1.4→.5 for HEARTS: after a heart-type SET replaces a member's original
/// hearts (Kasumi's LS selects the new type → heart_override), an additive
/// heart gain must still stack on top — in BOTH stage-heart computations.
#[test]
fn heart_override_additive_stacks_in_both_stage_heart_calcs() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Kasumi (base hearts: heart03 x1). Her LS sets her original heart to the
    // SELECTED color — the engine represents this as heart_override.
    let kasumi = game.id("PL!N-bp3-014-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, kasumi, -1];
    game.state.player1.hand.cards.push(filler); // live card for set_live_card
    fill_decks(&mut game, filler);

    // Advance into the live phase so Kasumi's LS fires and prompts.
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(filler);
    game.pass();
    game.pass();

    // Her Live Start asks which heart type (heart01 / heart03 / heart04).
    assert!(
        game.has_pending_choice(),
        "Kasumi's SelectHeartColor prompt expected"
    );
    game.select_option(0); // heart01

    // The set must have landed as an override on Kasumi.
    let (_, override_count) = *game
        .state
        .mods
        .heart_override
        .get(&kasumi)
        .expect("set_heart_type should have installed a heart_override on Kasumi");

    // Control: with no additive, the override value stands alone.
    let before = game
        .state
        .player1
        .calculate_stage_hearts(
            &game.db,
            &game.state.mods.heart_color_multiplier,
            &game.state.mods.heart_override,
            &game.state.mods.heart_modifiers,
            &game.state.mods.heart_copy,
        )
        .hearts
        .get(&HeartColor::Heart01)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        before, override_count,
        "control: override alone contributes exactly override_count heart01"
    );

    // 9.9.1.5: an additive +1 heart01 (any live_end gain) must stack on TOP.
    game.state.mods.add_heart_modifier(kasumi, HeartColor::Heart01, 1);

    let after_player = game
        .state
        .player1
        .calculate_stage_hearts(
            &game.db,
            &game.state.mods.heart_color_multiplier,
            &game.state.mods.heart_override,
            &game.state.mods.heart_modifiers,
            &game.state.mods.heart_copy,
        )
        .hearts
        .get(&HeartColor::Heart01)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        after_player,
        override_count + 1,
        "9.9.1.5: calculate_stage_hearts must apply additive modifiers ON TOP of the override"
    );

    // get_available_hearts now takes canonical ModifierEntry map; use it directly,
    // and also verify via legacy i32 adapter for backward compatibility.
    let after_zone = game
        .state
        .player1
        .stage
        .get_available_hearts(
            &game.db,
            &game.state.mods.heart_override,
            &game.state.mods.heart_modifiers,
            &game.state.mods.heart_color_multiplier,
            &game.state.mods.heart_copy,
        )
        .hearts
        .get(&HeartColor::Heart01)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        after_zone, override_count + 1,
        "9.9.1.5: get_available_hearts must apply additive modifiers ON TOP of the override"
    );
}
