use crate::helpers::*;

// ====================================================================
// 桂城 泉 (PL!HS-pb1-016-R / PL!HS-pb1-016-P＋) ab#0 — debut ability
// ====================================================================
// 桂城 泉 (PL!HS-pb1-016-R / PL!HS-pb1-016-P＋) ab#0 — debut ability
// ====================================================================
// Ability text:
//   {{toujyou.png|登場}}自分のステージにいるこのメンバー以外の
//   {{heart_06.png|heart06}}を持つメンバー1人は、ライブ終了時まで、
//   {{heart_06.png|heart06}}を得る。
//
// Translation:
//   Debut: Until end of live, 1 member on your stage other than this
//   member that has heart06 gains heart06.
//
// Key: The effect MUST filter targets to only those ALREADY possessing
// heart06. Members without heart06 should NOT receive it.
//
// Note: 登場 abilities trigger when the card is played to stage via
// play_to_stage(). They cannot be manually activated via activate_ability().
// ====================================================================

fn fill_decks(game: &mut TestGame, filler: i16) {
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// Scenario: activator + target_with_heart06 + filler_without_heart06
/// The target (has heart06 base) should receive +1 heart06 modifier.
/// The filler (no heart06) should NOT receive any modifier.
/// The activator is excluded by exclude_self.
#[test]
fn izumi_grants_heart06_to_member_with_heart06_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-pb1-016-R");
    let target = game.id("PL!-sd1-001-SD"); // has heart06=1 base_heart
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    // Pre-place target on Left and filler on Right (Center stays empty for izumi)
    game.state.player1.stage.stage = [target, -1, filler];
    game.give_energy(2);
    game.add_to_hand(izumi);

    // Play izumi to stage center — triggers 登場 ability
    game.play_to_stage(izumi, rabuka_engine::zones::MemberArea::Center);

    // Resolve any pending choices (e.g. SelectTarget)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let h_target = game
        .state
        .mods
        .get_heart_modifier(target, rabuka_engine::card::HeartColor::Heart06);
    let h_filler = game
        .state
        .mods
        .get_heart_modifier(filler, rabuka_engine::card::HeartColor::Heart06);
    let h_activator = game
        .state
        .mods
        .get_heart_modifier(izumi, rabuka_engine::card::HeartColor::Heart06);

    assert_eq!(
        h_target, 1,
        "Target (has heart06 base) should receive +1 heart06 modifier"
    );
    assert_eq!(
        h_filler, 0,
        "Filler (no heart06) should NOT receive heart06 modifier"
    );
    assert_eq!(
        h_activator, 0,
        "Activator should NOT receive heart06 modifier (excluded)"
    );
}

/// Scenario: only activator (has heart06) + 2 fillers (no heart06) on stage
/// No valid targets exist (no other member has heart06), so no one gets modifier.
#[test]
fn izumi_no_valid_target_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let izumi = game.id("PL!HS-pb1-016-R");
    let filler1 = game.id("PL!-sd1-010-SD");
    let filler2 = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler1);

    // Pre-place fillers on Left and Right (Center stays empty for izumi)
    game.state.player1.stage.stage = [filler1, -1, filler2];
    game.give_energy(2);
    game.add_to_hand(izumi);

    // Play izumi to stage center — triggers 登場 ability
    game.play_to_stage(izumi, rabuka_engine::zones::MemberArea::Center);

    // Resolve any pending choices
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let h_activator = game
        .state
        .mods
        .get_heart_modifier(izumi, rabuka_engine::card::HeartColor::Heart06);
    let h_filler1 = game
        .state
        .mods
        .get_heart_modifier(filler1, rabuka_engine::card::HeartColor::Heart06);
    let h_filler2 = game
        .state
        .mods
        .get_heart_modifier(filler2, rabuka_engine::card::HeartColor::Heart06);

    assert_eq!(
        h_activator, 0,
        "Activator should not receive heart06 (excluded)"
    );
    assert_eq!(
        h_filler1, 0,
        "Filler 1 (no heart06) should not receive heart06"
    );
    assert_eq!(
        h_filler2, 0,
        "Filler 2 (no heart06) should not receive heart06"
    );
}
