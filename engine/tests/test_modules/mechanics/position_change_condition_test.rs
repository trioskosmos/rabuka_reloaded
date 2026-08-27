use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn give_energy_and_deck(game: &mut TestGame) {
    game.give_energy(35);
    for _ in 0..20 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
}

fn play_two(game: &mut TestGame, c1: i16, p1: MemberArea, c2: i16, p2: MemberArea) {
    give_energy_and_deck(game);
    game.add_to_hand(c1);
    game.add_to_hand(c2);
    game.try_play_to_stage(c1, p1).unwrap();
    game.try_play_to_stage(c2, p2).unwrap();
}

fn play_three(
    game: &mut TestGame,
    c1: i16,
    p1: MemberArea,
    c2: i16,
    p2: MemberArea,
    c3: i16,
    p3: MemberArea,
) {
    give_energy_and_deck(game);
    game.add_to_hand(c1);
    game.add_to_hand(c2);
    game.add_to_hand(c3);
    game.try_play_to_stage(c1, p1).unwrap();
    game.try_play_to_stage(c2, p2).unwrap();
    game.try_play_to_stage(c3, p3).unwrap();
}

fn trigger_position_change_via_kinako(game: &mut TestGame, target_stage_area: &str) {
    let kinako_idx = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .position(|&id| {
            id != -1
                && game
                    .state
                    .card_database
                    .get_card(id)
                    .is_some_and(|c| c.card_no.contains("PL!SP-bp5-006"))
        })
        .expect("きな子 not found on stage");
    let kinako_id = game.state.player1.stage.stage[kinako_idx];
    game.activate_ability(kinako_id);
    game.drain_auto_ability_choices();
    assert!(
        game.has_pending_choice(),
        "Expected position choice after kidou activation"
    );
    let actions = game.generated_actions();
    let target_idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == target_stage_area)
        })
        .expect("Target position not found");
    game.select_generated(target_idx);
    game.drain_auto_ability_choices();
}

/// Syncri5e member moves to center → Syncri5e conditional gains 4 blades.
#[test]
fn syncrise_member_moves_to_center_triggers_blade_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let conditional = game.id("PL!SP-pb2-022-R");
    let kinako = game.id("PL!SP-bp5-006-R");
    play_two(
        &mut game,
        conditional,
        MemberArea::LeftSide,
        kinako,
        MemberArea::Center,
    );
    let before = game.state.mods.get_blade_modifier(conditional);
    let kinako_before = game.state.mods.get_blade_modifier(kinako);
    trigger_position_change_via_kinako(&mut game, "left");
    let after = game.state.mods.get_blade_modifier(conditional);
    let kinako_after = game.state.mods.get_blade_modifier(kinako);
    assert!(
        after >= before + 4,
        "Syncri5e member → center → gain 4 blades (was {}, now {})",
        before,
        after
    );
    assert_eq!(
        kinako_after, kinako_before,
        "Kinako (the moved member) should NOT gain blades — only the ability card gains them"
    );
}

/// Another 5yncri5e! member (not the ability card) moves to center → ability card gains blades.
#[test]
fn other_syncrise_member_moves_to_center_ability_card_gains_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let conditional = game.id("PL!SP-pb2-022-R");
    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!SP-bp1-014-N"); // 5yncri5e! member with no auto ability
    play_three(
        &mut game,
        conditional,
        MemberArea::LeftSide,
        filler,
        MemberArea::Center,
        kinako,
        MemberArea::RightSide,
    );
    let before = game.state.mods.get_blade_modifier(conditional);
    let filler_before = game.state.mods.get_blade_modifier(filler);
    let kinako_before = game.state.mods.get_blade_modifier(kinako);
    // Swap きな子 (right) → center. きな子 (5yncri5e!) moves TO center.
    // conditional stays at left. The ability on conditional should fire.
    game.activate_ability(kinako);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let center_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("center"))
        .expect("center position not found");
    game.select_generated(center_idx);
    game.drain_auto_ability_choices();
    let after = game.state.mods.get_blade_modifier(conditional);
    let filler_after = game.state.mods.get_blade_modifier(filler);
    let kinako_after = game.state.mods.get_blade_modifier(kinako);
    assert!(
        after >= before + 4,
        "Ability card should gain 4 blades when another 5yncri5e! member moves to center"
    );
    assert_eq!(
        filler_after, filler_before,
        "Filler (moved away from center) should NOT gain blades"
    );
    assert_eq!(
        kinako_after, kinako_before,
        "Kinako (moved TO center) should NOT gain blades — only the ability card gains them"
    );
}

/// non-Syncri5e member moves to center → Syncri5e conditional does NOT fire.
/// Uses a non-Syncri5e kidou mover (Shiki PL!SP-bp2-008-R, Liella!) to move μ's filler TO center.
/// Previous stub was lazy (no movement); now exercises real position_change: Shiki Center→Right swaps filler Right→Center.
#[test]
fn non_syncrise_member_moves_to_center_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let conditional = game.id("PL!SP-pb2-022-R"); // 5yncri5e! needs Syncri5e to center
    let filler = game.id("PL!-sd1-010-SD"); // μ's Printemps, NOT Syncri5e
    let shiki = game.id("PL!SP-bp2-008-R"); // 若菜四季, Liella! (non-Syncri5e) generic position swap
    // Setup: conditional Left, filler Right, shiki Center. Shiki's kidou: choose another area, swap.
    play_three(
        &mut game,
        conditional,
        MemberArea::LeftSide,
        filler,
        MemberArea::RightSide,
        shiki,
        MemberArea::Center,
    );
    let before = game.state.mods.get_blade_modifier(conditional);
    // Activate Shiki at Center, choose Right (= filler's area). Result: Shiki Center→Right, filler Right→Center (μ's TO center)
    game.activate_ability(shiki);
    game.drain_auto_ability_choices();
    assert!(
        game.has_pending_choice(),
        "Shiki position change should offer target areas"
    );
    let actions = game.generated_actions();
    let target_idx = actions
        .iter()
        .position(|a| {
            a.parameters
                .as_ref()
                .and_then(|p| p.stage_area.as_deref())
                .is_some_and(|area| area == "right" || area == "right_side")
        })
        .expect("Right target not found for Shiki");
    game.select_generated(target_idx);
    game.drain_auto_ability_choices();
    // Verify swap: filler should now be at Center, shiki at Right
    assert_eq!(
        game.state.player1.stage.stage[1], filler,
        "filler (μ's) should have moved TO center"
    );
    assert_eq!(
        game.state.player1.stage.stage[2], shiki,
        "shiki should have moved to Right"
    );
    let after = game.state.mods.get_blade_modifier(conditional);
    assert_eq!(
        after, before,
        "non-Syncri5e member (μ's) moved TO center → Syncri5e conditional must NOT gain 4 blades (was {}, now {})",
        before, after
    );
}

/// Syncri5e member moves to non-center → conditional does NOT trigger.
#[test]
fn syncrise_member_moves_to_non_center_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let conditional = game.id("PL!SP-pb2-022-R");
    let kinako = game.id("PL!SP-bp5-006-R");
    let syncrise_abilityless = game.id("PL!SP-bp1-014-N");
    play_three(
        &mut game,
        conditional,
        MemberArea::Center,
        syncrise_abilityless,
        MemberArea::LeftSide,
        kinako,
        MemberArea::RightSide,
    );
    let before = game.state.mods.get_blade_modifier(conditional);
    // Swap きな子 (right) ↔ left (Syncri5e abilityless).
    // abilityless moves left→right (not center). conditional at center unaffected.
    game.activate_ability(kinako);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let left_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("left"))
        .expect("left position not found");
    game.select_generated(left_idx);
    game.drain_auto_ability_choices();
    let after = game.state.mods.get_blade_modifier(conditional);
    assert_eq!(
        after, before,
        "Syncri5e member moved to non-center → should NOT gain blades"
    );
}

/// Self-target card moves → its own ability triggers (gain 1 blade).
#[test]
fn self_target_card_moves_triggers_own_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let self_target = game.id("PL!SP-sd2-011-SD2");
    let kinako = game.id("PL!SP-bp5-006-R");
    play_two(
        &mut game,
        self_target,
        MemberArea::LeftSide,
        kinako,
        MemberArea::Center,
    );
    let before = game.state.mods.get_blade_modifier(self_target);
    trigger_position_change_via_kinako(&mut game, "left");
    let after = game.state.mods.get_blade_modifier(self_target);
    assert!(
        after >= before + 1,
        "Self-target moved → gain 1 blade (was {}, now {})",
        before,
        after
    );
}

/// Other card moves → self-target card's ability does NOT trigger.
#[test]
fn other_card_moves_self_target_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let self_target = game.id("PL!SP-sd2-011-SD2");
    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    play_three(
        &mut game,
        self_target,
        MemberArea::LeftSide,
        filler,
        MemberArea::Center,
        kinako,
        MemberArea::RightSide,
    );
    let before = game.state.mods.get_blade_modifier(self_target);
    // Swap きな子 (right) ↔ center (filler). self_target at left stays put.
    game.activate_ability(kinako);
    game.drain_auto_ability_choices();
    let actions = game.generated_actions();
    let center_idx = actions
        .iter()
        .position(|a| a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) == Some("center"))
        .expect("center position not found");
    game.select_generated(center_idx);
    game.drain_auto_ability_choices();
    let after = game.state.mods.get_blade_modifier(self_target);
    assert_eq!(
        after, before,
        "Self-target did not move → should NOT gain blade"
    );
}

/// No position change → neither ability triggers.
#[test]
fn no_position_change_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let conditional = game.id("PL!SP-pb2-022-R");
    let self_target = game.id("PL!SP-sd2-011-SD2");
    // Just play both cards, no position change
    give_energy_and_deck(&mut game);
    game.add_to_hand(conditional);
    game.add_to_hand(self_target);
    game.try_play_to_stage(conditional, MemberArea::Center)
        .unwrap();
    game.try_play_to_stage(self_target, MemberArea::LeftSide)
        .unwrap();
    let cond_before = game.state.mods.get_blade_modifier(conditional);
    let self_before = game.state.mods.get_blade_modifier(self_target);
    assert!(
        !game.state.position_change_occurred_this_turn,
        "No position change should have occurred"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(conditional),
        cond_before,
        "No position change → conditional should NOT gain blades"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(self_target),
        self_before,
        "No position change → self-target should NOT gain blade"
    );
}
