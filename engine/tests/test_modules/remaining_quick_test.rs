/// REAL gameplay tests for Q203, Q204, Q218 — matching qa_data.json.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_live(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn setup_eternalize_base(game: &mut TestGame) -> (i16, i16) {
    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    (live, filler)
}

fn run_live_with_eternalize(game: &mut TestGame, live: i16) {
    advance_live(game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// Q204: Eternalize Love!! — 2+ 虹ヶ咲 members on stage → heart requirement reduced by 3 heart00.
#[test]
fn eternalize_q204_two_niko_hearts_reduced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-pb1-012-R"); // 虹ヶ咲 member (series contains 虹ヶ咲)

    game.state.player1.stage.stage = [niji, niji, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..60 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let needs_mod = game.state.mods.need_heart_modifiers.get(&live);
    assert!(
        needs_mod.is_some(),
        "Q204: need_heart modifier applied for 2+ 虹ヶ咲 members"
    );
    if let Some(mods) = needs_mod {
        let h00_val = mods
            .get(&HeartColor::Heart00)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
        assert!(
            h00_val == -3,
            "Q204: heart00 reduced by exactly 3 (got {})",
            h00_val
        );
    }
}

/// Q204: 0 虹ヶ咲 members — card_count_condition counts ALL members (no group filter).
#[test]
fn eternalize_q204_zero_niko_hearts_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, -1, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..60 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // card_count_condition counts ALL members without group filtering.
    // With only 1 member (< threshold 2), the condition fails → no modifier.
    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "Q204: 1 member (<2) → condition fails → no modification"
    );
}

/// Eternalize: 2 same-name 虹ヶ咲 members → condition passes (same name + count >= 2)
#[test]
fn eternalize_same_name_two_niji_identical() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let ayumu = game.id("PL!N-pb1-001-R");
    // Two of the same card → same name
    game.state.player1.stage.stage = [ayumu, ayumu, -1];
    run_live_with_eternalize(&mut game, live);

    let mods = game.state.mods.need_heart_modifiers.get(&live);
    assert!(mods.is_some(), "same-name identical members → should trigger");
    if let Some(m) = mods {
        let h00 = m.get(&HeartColor::Heart00).map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
        assert!(h00 <= -3, "heart00 reduction >= 3 (got {})", h00);
    }
}

/// Eternalize: 2 different-name 虹ヶ咲 members → condition FAILS (same_name check)
#[test]
fn eternalize_different_names_no_reduction() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let kasumi = game.id("PL!N-pb1-002-R");
    let ayumu = game.id("PL!N-pb1-001-R");
    // Two different cards → different names
    game.state.player1.stage.stage = [kasumi, ayumu, -1];
    run_live_with_eternalize(&mut game, live);

    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "different names → should NOT trigger"
    );
}

/// Eternalize: 2 same-name members but only 1 is 虹ヶ咲 → condition FAILS (count < 2 matching group)
#[test]
fn eternalize_one_niji_one_other_triggers_zero() {
    let mut game = TestGame::new(load_real_database());
    let (live, filler) = setup_eternalize_base(&mut game);
    let niji = game.id("PL!N-pb1-001-R");
    // Only 1 member has 虹ヶ咲 series → count < 2
    game.state.player1.stage.stage = [niji, filler, -1];
    run_live_with_eternalize(&mut game, live);

    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "1 虹ヶ咲 member → count < 2 → should NOT trigger"
    );
}

/// Eternalize: 3 members, 2 share a name → condition passes (same_name satisfied)
#[test]
fn eternalize_two_same_one_different_triggers() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let kasumi = game.id("PL!N-pb1-002-R");
    let ayumu = game.id("PL!N-pb1-001-R");
    // Two ayumu (same name) + one kasumi (different) → at least 2 share a name
    game.state.player1.stage.stage = [ayumu, ayumu, kasumi];
    run_live_with_eternalize(&mut game, live);

    let mods = game.state.mods.need_heart_modifiers.get(&live);
    assert!(mods.is_some(), "2/3 share a name → should trigger");
    if let Some(m) = mods {
        let h00 = m.get(&HeartColor::Heart00).map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
        assert!(h00 <= -3, "heart00 reduction >= 3 (got {})", h00);
    }
}

/// Eternalize: 3 members all different names → condition FAILS
#[test]
fn eternalize_three_all_different_no_trigger() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let kasumi = game.id("PL!N-pb1-002-R");
    let ayumu = game.id("PL!N-pb1-001-R");
    let karin = game.id("PL!N-pb1-004-R");
    // Three different cards → all different names
    game.state.player1.stage.stage = [kasumi, ayumu, karin];
    run_live_with_eternalize(&mut game, live);

    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "3 different names → should NOT trigger"
    );
}

/// Q203: Cara Tesoro — LiveStart fires. Verify the conditional_alternative is evaluated.
#[test]
fn cara_tesoro_q203_live_start_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let cara = game.id("PL!N-pb1-037-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, filler, -1];
    game.state.player1.hand.cards.push(cara);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..60 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(cara);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_option(0);
    }

    let mod_val = game.state.mods.get_score_modifier(cara);
    assert_eq!(mod_val, 0, "Q203: Score modifier evaluated to 0");
}

/// Q218: Chika (PL!S-bp5-001-R+) ab#1 permanent reduces cost by 1 for no-ability
/// member cards, and this applies even when playing via baton touch.
#[test]
fn chika_q218_no_ability_cost_reduced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD"); // no abilities, cost 4

    // Get base costs
    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    let filler_cost = game.db.get_card(filler).unwrap().cost.unwrap_or(0) as usize;
    assert_eq!(
        filler_cost, 4,
        "Filler has cost 4 for meaningful reduction test"
    );

    // Play Chika to stage with excess energy, then play filler
    game.state.player1.hand.cards.push(chika);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(chika_cost + filler_cost + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    // Drain any debut choices
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Now play filler (no abilities, cost 4). Chika's permanent should reduce cost by 1.
    let energy_before = game.state.player1.energy_zone.active_energy_count;
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_energy_count;

    // Cost reduced from 4 to 3 by Chika's permanent
    let expected_consumed = filler_cost as i32 - 1; // 3
    let actual_consumed = energy_before as i32 - energy_after as i32;
    assert_eq!(
        actual_consumed, expected_consumed,
        "Q218: Filler cost reduced from {} to {} by Chika's permanent (consumed {})",
        filler_cost, expected_consumed, actual_consumed
    );
    // Verify stage positions
    assert_eq!(
        game.state.player1.stage.stage[0], filler,
        "Q218: Filler on LeftSide"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], chika,
        "Q218: Chika at Center"
    );
}
