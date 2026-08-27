/// Untested-abilities batch 30 — 常時 thresholds + ライブ成功時 conditional draws.
/// Sibling idioms copied from wien_cost_mod_test (recalculate_constants +
/// get_cost_modifier), b7_constant_ability_test (get_heart_modifier),
/// untested_abilities_batch15 (fire_trigger), position_change_triggers_jidou_move
/// (real area-move via きなこ's kidou swap).
///
/// - PL!SP-pb2-027-N 葉月恋 (常時): energy ≥6 → heart03, ≥8 → another heart03.
///   Boundaries 5/6/7/8 all pinned.
/// - PL!N-sd2-003-SD2 桜坂しずく (常時): hand cost −2 while a 『虹ヶ咲』 live card
///   is in the own success zone.
/// - PL!-bp6-023-L sweet&sweet holiday (ライブ成功時): draw 1; μ's card in own
///   success zone → draw 1 more.
/// - PL!SP-sd2-003-SD2 嵐千砂都 (ライブ成功時): draw 1; this member area-moved
///   this turn → draw 1 more.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

/// Thin wrapper: helpers::fire_trigger plus the choice drain this batch's
/// constant-threshold assertions rely on (the one behavioral difference from
/// the shared helper — kept local and explicit).
fn fire_trigger(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trig: &str) {
    crate::helpers::fire_trigger(game, cid, trigger, trig);
    game.drain_auto_ability_choices();
}

// ====================================================================
// PL!SP-pb2-027-N 葉月恋 — dual energy thresholds on heart03 constants
// ====================================================================

fn ren_energy_setup(game: &mut TestGame, energy: usize) -> i16 {
    let ren = game.id("PL!SP-pb2-027-N");
    // Direct stage placement: we are testing the constant re-evaluation, not
    // the debut pipeline.
    game.state.player1.stage.stage[1] = ren;
    game.give_energy(energy);
    game.state.recalculate_constants();
    ren
}

#[test]
fn pb2_027_five_energy_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_energy_setup(&mut game, 5);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        0,
        "5 energy (<6) -> no heart03"
    );
}

#[test]
fn pb2_027_six_energy_one_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_energy_setup(&mut game, 6);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        1,
        "6 energy (>=6, <8) -> exactly one heart03"
    );
}

#[test]
fn pb2_027_seven_energy_still_one_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_energy_setup(&mut game, 7);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        1,
        "7 energy (<8) -> still one heart03"
    );
}

#[test]
fn pb2_027_eight_energy_two_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = ren_energy_setup(&mut game, 8);
    assert_eq!(
        game.state.mods.get_heart_modifier(ren, HeartColor::Heart03),
        2,
        "8 energy (>=8) -> both heart03"
    );
}

// ====================================================================
// PL!N-sd2-003-SD2 桜坂しずく — hand cost −2 gated on own success zone
// ====================================================================

#[test]
fn sd2_003_hand_cost_reduced_with_nijigasaki_live_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-sd2-003-SD2");
    let niji_live = game.id("PL!N-sd1-025-SD"); // 『虹ヶ咲』 live card
    game.add_to_hand(shizuku);
    game.state.player1.success_live_card_zone.cards.push(niji_live);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(shizuku),
        -2,
        "虹ヶ咲 live card in success zone -> hand cost −2"
    );
}

#[test]
fn sd2_003_hand_cost_full_without_success_zone_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-sd2-003-SD2");
    game.add_to_hand(shizuku);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(shizuku),
        0,
        "empty success zone -> full hand cost"
    );
}

#[test]
fn sd2_003_non_nijigasaki_live_does_not_reduce_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let shizuku = game.id("PL!N-sd2-003-SD2");
    // Aqours live card — wrong group for 『虹ヶ咲』 gate.
    let aqours_live = game.id("PL!-sd1-020-SD");
    game.add_to_hand(shizuku);
    game.state.player1.success_live_card_zone.cards.push(aqours_live);
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.get_cost_modifier(shizuku),
        0,
        "non-虹ヶ咲 live card in success zone -> full hand cost"
    );
}

// ====================================================================
// PL!-bp6-023-L sweet&sweet holiday — ライブ成功時 conditional second draw
// ====================================================================

#[test]
fn bp6_023_draws_extra_with_mus_live_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-023-L");
    let mus_live = game.id("PL!-sd1-020-SD"); // 『μ's』 live card

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.success_live_card_zone.cards.push(mus_live);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "μ's card in success zone -> draw 1 + 1 more"
    );
}

#[test]
fn bp6_023_single_draw_without_success_zone_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-023-L");

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "no card in success zone -> only the base draw"
    );
}

#[test]
fn bp6_023_non_mus_live_in_success_zone_single_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-bp6-023-L");
    let niji_live = game.id("PL!N-sd1-025-SD"); // wrong group

    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(live);
    game.state.player1.success_live_card_zone.cards.push(niji_live);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "non-μ's live card in success zone -> no extra draw"
    );
}

// ====================================================================
// PL!SP-sd2-003-SD2 嵐千砂都 — ライブ成功時 gated on own area move this turn
// ====================================================================

/// Play 千砂都 (left) + きなこ (right), then perform きなこ's real kidou
/// position-change swap so 千砂都 has genuinely area-moved this turn.
fn chisato_area_move_this_turn(game: &mut TestGame, do_swap: bool) -> i16 {
    let chisato = game.id("PL!SP-sd2-003-SD2");
    let kinako = game.id("PL!SP-bp5-006-R");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.add_to_hand(chisato);
    game.add_to_hand(kinako);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(30);

    game.try_play_to_stage(chisato, MemberArea::LeftSide).expect("play chisato");
    game.try_play_to_stage(kinako, MemberArea::RightSide).expect("play kinako");

    if do_swap {
        game.activate_ability(kinako);
        assert!(game.has_pending_choice(), "expected position|destination choice");
        // Pick the generated ChoicePosition action pointing at LEFT (chisato's area).
        let actions = game.generated_actions();
        let left_action = actions
            .iter()
            .find(|a| {
                a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition
                    && a.description.contains("左")
            })
            .or_else(|| {
                actions.iter().find(|a| {
                    a.action_type == rabuka_engine::game_setup::ActionType::ChoicePosition
                })
            })
            .expect("a position option must exist");
        let p = left_action.parameters.as_ref().expect("params");
        rabuka_engine::turn::TurnEngine::resume_with_choice(
            &mut game.state,
            p.card_id,
            p.card_indices.clone(),
        )
        .expect("position change failed");
        assert_eq!(
            game.state.player1.stage.stage[2], chisato,
            "swap moved 千砂都 to the right side"
        );
    }
    chisato
}

#[test]
fn sd2_003_draws_extra_after_area_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = chisato_area_move_this_turn(&mut game, true);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, chisato, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        2,
        "area-moved this turn -> draw 1 + 1 more"
    );
}

#[test]
fn sd2_003_single_draw_without_area_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let chisato = chisato_area_move_this_turn(&mut game, false);

    let deck_before = game.state.player1.main_deck.cards.len();
    fire_trigger(&mut game, chisato, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert_eq!(
        deck_before - game.state.player1.main_deck.cards.len(),
        1,
        "no area move this turn -> only the base draw"
    );
}
