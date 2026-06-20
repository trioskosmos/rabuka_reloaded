/// Tests that a position_change swap correctly triggers 自動 movement_condition
/// auto-abilities (e.g. 嵐 千砂都: when this member moves area, put 1 energy card
/// from energy deck in wait state).
///
/// Scenario:
///   1. Play 桜小路きな子 (PL!SP-bp5-006-R) and 嵐 千砂都 (PL!SP-bp2-003-R)
///      to stage in main phase.
///   2. Activate きな子's kidou ability: cost = top 3 deck → discard,
///      effect = position_change (swap with another member).
///   3. Select the area where 千砂都 is sitting as the swap destination.
///   4. The swap occurs. 千砂都 has now moved areas.
///   5. 千砂都's jidou auto-ability should fire: put 1 energy card from energy
///      deck in wait state.
use crate::helpers::*;

/// Advance to MainPhase.


#[test]
fn position_change_swap_triggers_jidou_energy_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kinako = game.id("PL!SP-bp5-006-R"); // 桜小路きな子 (kidou: position_change)
    let chisato = game.id("PL!SP-bp2-003-R"); // 嵐 千砂都 (jidou: moved → energy draw)
    let filler = game.id("PL!-sd1-010-SD"); // generic member filler for deck

    // Place both cards in hand so we can play them to stage
    game.add_to_hand(kinako);
    game.add_to_hand(chisato);
    // Populate deck so top-3 cost can be paid
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // Populate energy deck so 千砂都's energy draw can succeed
    for _ in 0..10 {
        game.state.player1.energy_deck.cards.push(game.id("LL-E-001-SD"));
    }
    game.give_energy(20);

    // Play 千砂都 to left, きな子 to right
    let r1 = game.try_play_to_stage(chisato, rabuka_engine::zones::MemberArea::LeftSide);
    eprintln!("[PLAY1] chisato→left: {:?}", r1);
    r1.expect("play chisato to left failed");
    let r2 = game.try_play_to_stage(kinako, rabuka_engine::zones::MemberArea::RightSide);
    eprintln!("[PLAY2] kinako→right: {:?}", r2);
    r2.expect("play kinako to right failed");

    // Sanity: stage layout before swap
    assert_eq!(game.state.player1.stage.stage[0], chisato, "千砂都 at left");
    assert_eq!(game.state.player1.stage.stage[2], kinako, "きな子 at right");

    // Activate きな子's kidou ability (ab#0: position change)
    game.activate_ability(kinako);

    // The cost is "deck top 3 → discard" (auto-executed), then the effect
    // creates a position|destination choice.
    assert!(
        game.has_pending_choice(),
        "Expected position|destination choice after activating きな子's ability"
    );

    // Find the "left" position option (where 千砂都 currently is) among the
    // generated ChoicePosition actions and select it.
    let actions = game.generated_actions();
    let descriptions: Vec<&str> = actions
        .iter()
        .map(|a| a.description.as_str())
        .collect();
    eprintln!("[POSITION_OPTIONS] {:?}", descriptions);

    let left_idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == "left")
        })
        .unwrap_or_else(|| {
            panic!(
                "No 'left' position option found. Available: {:?}",
                descriptions
            )
        });

    // Select left (swap きな子 → left, 千砂都 → right)
    game.select_generated(left_idx);

    // After the swap, 千砂都 may have an optional jidou auto-ability choice
    // (SelectAutoAbility). Drain those if present.
    game.drain_auto_ability_choices();

    // Verify: the physical swap happened.
    assert_eq!(
        game.state.player1.stage.stage[0], kinako,
        "きな子 should now be on the left after swap"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], chisato,
        "千砂都 should now be on the right after swap"
    );

    // Verify: movement was recorded for both cards
    assert!(
        game.state.has_card_moved_this_turn(kinako),
        "きな子 should be recorded as moved"
    );
    assert!(
        game.state.has_card_moved_this_turn(chisato),
        "千砂都 should be recorded as moved"
    );

    // Verify: position_change flag is set
    assert!(
        game.state.position_change_occurred_this_turn,
        "position_change_occurred_this_turn should be true"
    );

    // KEY ASSERTION: 千砂都's jidou auto-ability should have placed 1 energy
    // card from the energy deck into the energy zone (in wait state).
    let expected_energy_count = 20 + 1;
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        expected_energy_count,
        "千砂都's auto-ability should have placed 1 energy card in energy zone, \
         but energy zone has {} cards (expected {})",
        game.state.player1.energy_zone.cards.len(),
        expected_energy_count
    );
}
