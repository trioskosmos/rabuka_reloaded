use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

/// Helper: advance through 5 pass phases to reach live start
fn advance_to_live_start(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Helper: fill both players' decks with filler cards
fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

// ============================================================
// Card: PL!N-pb1-006-R — 近江彼方 (Konoe Kanata)
// Ability: 起動 — このメンバーをウェイトにする：エネルギーを1枚アクティブにする。
//   Cost:  change_state (wait self)
//   Effect: gain_resource (activate 1 energy)
// ============================================================

/// Happy path: play card to stage via normal gameplay, activate ability,
/// verify both cost payment (wait state) and effect (energy +1).
#[test]
fn n_pb1_006_wait_self_activates_one_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!N-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    // -- Real gameplay sequence --
    // 1. Main phase: give player energy for later plays
    game.give_energy(10);
    // 2. Populate deck (required for some game operations)
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // 3. Draw card into hand
    game.state.player1.hand.cards.push(card);

    // 4. Play card from hand to stage (real TurnEngine action)
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_energy_count;

    // 5. Activate ability via UseAbility action
    game.activate_ability(card);

    // -- Assertions --
    // Cost: card should be in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some(&"wait".to_string()),
        "Cost: card must be in wait state after activation"
    );
    // Effect: energy increased by exactly 1
    let energy_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        energy_after,
        energy_before + 1,
        "Effect: should activate exactly 1 energy (was {}, now {})",
        energy_before,
        energy_after
    );
}

/// Edge case: ability activates energy from wait to active state.
/// With only wait-energy (no active), activation should convert one.
#[test]
fn n_pb1_006_activates_wait_energy_to_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!N-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15); // card costs 9 to play
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Convert all active energy to wait, so we have only wait-energy
    game.state.player1.energy_zone.active_energy_count = 0;

    let active_before = game.state.player1.energy_zone.active_energy_count;
    let total_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(card);

    // Cost paid
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some(&"wait".to_string()),
        "Cost paid: card must be wait"
    );
    // Effect: one wait-energy card activated
    let active_after = game.state.player1.energy_zone.active_energy_count;
    assert_eq!(
        active_after,
        active_before + 1,
        "Effect: should activate exactly 1 energy (was {} active, now {})",
        active_before,
        active_after
    );
    // Total energy cards unchanged (activation doesn't create/destroy cards)
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        total_before,
        "Energy card count unchanged"
    );
}

/// Edge case: no turn limit on this ability, so it could be activated
/// multiple times. Each activation waits the card and gives 1 energy.
#[test]
fn n_pb1_006_can_activate_multiple_times() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!N-pb1-006-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15); // card costs 9 to play
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let energy_before = game.state.player1.energy_zone.active_energy_count;

    // Activate 3 times
    for i in 0..3 {
        game.activate_ability(card);
        assert_eq!(
            game.state.player1.energy_zone.active_energy_count,
            energy_before + i + 1,
            "Activation {} should give 1 energy",
            i + 1
        );
    }

    // Card is in wait state after all activations
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some(&"wait".to_string()),
        "Card should be wait after multiple activations"
    );
}

// ============================================================
// Card: PL!SP-bp5-016-N — 葉月 恋 (Hazuki Ren)
// Ability: 常時 — 自分のエネルギーが10枚以上あるかぎり、
//           {{heart_06.png|heart06}}{{heart_06.png|heart06}}を得る。
//   Effect: gain_resource(heart06 ×2) while energy_zone.count >= 10
// ============================================================

/// Condition met: give 10+ energy, recalculate constants,
/// verify card gains heart06 ×2.
#[test]
fn sp_bp5_016_energy_ge_10_grants_heart06_x2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    // Play card to stage first (costs 9 energy)
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // After playing, we still have 15 - 9 = 6 energy.
    // Give 4 more to reach 10.
    game.give_energy(4);
    assert!(
        game.state.player1.energy_zone.active_energy_count >= 10,
        "Precondition: energy >= 10"
    );

    // Trigger constant ability evaluation
    game.state.recalculate_constants();

    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert!(
        h06 >= 2,
        "Should gain at least heart06 ×2 with energy >= 10 (got {})",
        h06
    );
}

/// Condition unmet: energy < 10 should give 0 heart06 from this ability.
#[test]
fn sp_bp5_016_energy_lt_10_grants_no_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    // Play card to stage
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Ensure total energy cards < 10
    // (play cost 9 consumed 9 energy, but give_energy just adds)
    // Remove enough cards to drop below 10
    game.state.player1.energy_zone.cards.truncate(5);
    game.state.player1.energy_zone.active_energy_count = 5;
    assert!(
        game.state.player1.energy_zone.cards.len() < 10,
        "Precondition: total energy cards < 10 (got {})",
        game.state.player1.energy_zone.cards.len()
    );

    game.state.recalculate_constants();

    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert_eq!(
        h06, 0,
        "Should gain 0 heart06 with energy < 10 (got {})",
        h06
    );
}

/// Constant ability fires automatically during live start phase.
#[test]
fn sp_bp5_016_constant_evaluated_during_live_phase() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");
    let filler_live = game.id("PL!-sd1-019-SD");

    // Play card to stage
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Give extra energy to meet condition (total > 10)
    game.give_energy(5);

    // Set up live card for live phase
    game.state.player1.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(filler_live);

    // Advance through phases to live start
    advance_to_live_start(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    // Live start should have triggered constant evaluation
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert!(
        h06 >= 2,
        "Live phase: should gain heart06 ×2 from constant (got {})",
        h06
    );
}

// ============================================================
// Card: PL!HS-bp5-016-N — 桂城 泉 (Katsura Izumi)
// Abilities:
//   登場: 手札を1枚控え室に置いてもよい：相手のステージにいる
//         コスト4以下のメンバーを2人までウェイトにする。
//   常時: 相手のステージにウェイト状態のメンバーが2人以上いる
//         かぎり、{{heart_06.png|heart06}}を得る。
// ============================================================

/// Appear: play card to stage, optionally discard 1 to wait up to 2
/// opponent cost ≤4 members. Then constant: 2+ opponent wait → heart06.
#[test]
fn hs_bp5_016_wait_opponent_members_triggers_constant_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");
    let discard_target = game.id("PL!-sd1-019-SD");
    let opp_member = game.id("PL!-sd1-010-SD");

    // Setup: energy, decks
    game.give_energy(15);
    fill_decks(&mut game, filler);

    // Place opponent members on stage (cost ≤ 4)
    game.state.player2.stage.stage = [opp_member, opp_member, -1];

    // Put card and discard target in hand
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(discard_target);

    // Play card to stage → appear ability triggers
    game.play_to_stage(card, MemberArea::Center);

    // Should prompt: may discard 1 card to activate optional cost
    while game.has_pending_choice() {
        // Pay optional cost: discard 1 from hand
        game.select_indices(&[0]);
    }

    // Assert: opponent members are now in wait state
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp_member),
        Some(&"wait".to_string()),
        "Appear: opponent member should be waited"
    );

    // Assert: discard target was removed from hand
    assert!(
        !game.state.player1.hand.cards.contains(&discard_target),
        "Appear: discard target should be removed from hand"
    );

    // Now assert constant ability: 2+ opponent wait → heart06
    game.state.recalculate_constants();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert!(
        h06 >= 1,
        "Constant: should gain heart06 with 2+ opponent wait (got {})",
        h06
    );
}

/// Edge case: constant gives 0 heart06 when condition not met
/// (opponent has < 2 wait members).
#[test]
fn hs_bp5_016_no_heart06_without_opponent_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!HS-bp5-016-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    // Skip the optional discard (decline the ability)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // No opponent wait members
    game.state.recalculate_constants();
    let h06 = game
        .state
        .mods
        .get_heart_modifier(card, HeartColor::Heart06);
    assert_eq!(
        h06, 0,
        "Should get 0 heart06 without opponent wait members (got {})",
        h06
    );
}

// ============================================================
// Card: PL!-bp4-018-N — 矢澤にこ (Yazawa Nico)
// Ability: 常時 — 自分の成功ライブカード置き場にあるカードの
//           スコアの合計が相手より高いかぎり、
//           {{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
//   Effect: gain_resource(blade ×2) while own_score > opponent_score
// ============================================================

/// Own success score > opponent: gain blade ×2 from constant.
#[test]
fn bp4_018_own_success_score_greater_grants_blade_x2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");
    let own_live = game.id("PL!-sd1-019-SD"); // score 1
    let _opp_live = game.id("PL!-sd1-020-SD"); // score 2 (used in next test)

    // Play card to stage (cost 11)
    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Put own live card (score 1) in success zone, opponent has nothing
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live);
    // own score = 1, opponent score = 0 → own > opponent

    game.state.recalculate_constants();
    let blade = game.state.mods.get_blade_modifier(card);
    assert!(
        blade >= 2,
        "Should gain blade ×2 when own score {} > opponent score {} (got {})",
        1,
        0,
        blade
    );
}

/// Opponent score higher: own constant should give 0 blade.
#[test]
fn bp4_018_opponent_score_higher_grants_zero_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");
    let own_live = game.id("PL!-sd1-019-SD"); // score 1
    let opp_live = game.id("PL!-sd1-020-SD"); // score 2

    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Opponent has higher score
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live);
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opp_live);

    game.state.recalculate_constants();
    let blade = game.state.mods.get_blade_modifier(card);
    assert_eq!(
        blade, 0,
        "Should gain 0 blade when own score {} < opponent score {} (got {})",
        1, 2, blade
    );
}

/// Constant updates dynamically when scores change.
#[test]
fn bp4_018_blade_updates_when_score_changes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");
    let own_live_a = game.id("PL!-sd1-019-SD"); // score 1
    let own_live_b = game.id("PL!-sd1-019-SD"); // score 1
    let opp_live = game.id("PL!-sd1-020-SD"); // score 2

    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Initially no cards in either success zone → scores equal → 0 blade
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "Equal scores (0=0) should give 0 blade"
    );

    // Add own card: own 1 > opp 0 → gain blade ×2
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live_a);
    game.state.recalculate_constants();
    assert!(
        game.state.mods.get_blade_modifier(card) >= 2,
        "Own score 1 > opp score 0 should give blade ×2"
    );

    // Add opponent card: own 1 < opp 2 → lose blade
    game.state
        .player2
        .success_live_card_zone
        .cards
        .push(opp_live);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "Own score 1 < opp score 2 should give 0 blade"
    );

    // Add second own card: own 2 == opp 2 → still 0 (not strictly greater)
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(own_live_b);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_blade_modifier(card),
        0,
        "Equal scores (2=2) should give 0 blade"
    );
}

// ============================================================
// Card: PL!HS-bp5-021-L — ジョーシキーキラキュー (LIVE card)
// Abilities (LiveStart):
//   ab#0: ライブ終了まで、自分のステージにいる『蓮ノ空』のメンバー1人の
//          元々持つハートを全て{{heart_01.png|heart01}}にする。
//          → set_heart_type (heart01), target 1 蓮ノ空 member
//   ab#1: 自分のステージに『みらくらぱーく！』のメンバーが3人以上いる
//          場合、このカードのスコアを+1する。
//          → modify_score (+1, target=self) when ≥3 みらくらぱーく！ on stage
// ============================================================

/// Helper: advance through 5 pass phases to reach LiveCardSet phase
fn advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Single 蓮ノ空 member on stage: ab#0 converts that member's hearts to heart01.
#[test]
fn hs_bp5_021_single_hasunosora_member_heart_conversion() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!HS-bp5-021-L");
    let member = game.id("PL!HS-sd1-003-SD"); // みらくらぱーく！, hearts: heart01:1, heart05:1
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(member);
    game.play_to_stage(member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // heart_color_multiplier should be on the stage member, not the live card
    assert!(
        game.state.mods.heart_color_multiplier.contains_key(&member),
        "Member should have heart_color_multiplier"
    );
    assert_eq!(
        game.state.mods.heart_color_multiplier.get(&member),
        Some(&HeartColor::Heart01),
        "Member's hearts should be converted to heart01"
    );
    // Live card should NOT have the multiplier
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&live_card),
        "Live card should NOT have heart_color_multiplier"
    );

    // Stage heart calculation reflects the conversion
    let stage_hearts = game.state.player1.calculate_stage_hearts(
        &game.state.card_database,
        &game.state.mods.heart_color_multiplier,
        &Default::default(),
        &Default::default(),
    );
    // member has heart01:1, heart05:1 → 2 total, all become heart01
    let heart01 = stage_hearts
        .hearts
        .get(&HeartColor::Heart01)
        .copied()
        .unwrap_or(0);
    assert!(
        heart01 >= 2,
        "Should have ≥2 heart01 after conversion (got {})",
        heart01
    );
}

/// Multiple 蓮ノ空 members: player is prompted to choose one member for conversion.
#[test]
fn hs_bp5_021_multiple_hasunosora_members_choice_one_converted() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!HS-bp5-021-L");
    let member_a = game.id("PL!HS-sd1-001-SD"); // スリーズブーケ, heart04:3
    let member_b = game.id("PL!HS-sd1-003-SD"); // みらくらぱーく！, heart01:1, heart05:1
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(20);
    fill_decks(&mut game, filler);

    game.state.player1.hand.cards.push(member_a);
    game.state.player1.hand.cards.push(member_b);
    game.play_to_stage(member_a, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(member_b, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    // Should get a choice prompt to select which member to convert
    // Select the first eligible member (LeftSide = member_a)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Exactly one member should have the multiplier (the selected one)
    let count: usize = game.state.mods.heart_color_multiplier.len();
    assert_eq!(
        count, 1,
        "Exactly 1 member should have heart_color_multiplier, got {}",
        count
    );

    // The selected member (member_a, LeftSide = index 0) should have it
    assert!(
        game.state
            .mods
            .heart_color_multiplier
            .contains_key(&member_a),
        "Selected member (member_a) should have heart_color_multiplier"
    );
    // The unselected member should NOT have it
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&member_b),
        "Unselected member (member_b) should NOT have heart_color_multiplier"
    );
    // Live card should NOT have it
    assert!(
        !game
            .state
            .mods
            .heart_color_multiplier
            .contains_key(&live_card),
        "Live card should NOT have heart_color_multiplier"
    );
}

/// No 蓮ノ空 members on stage: ab#0 should be a true no-op (no eligible target).
#[test]
fn hs_bp5_021_no_hasunosora_members_noop() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!HS-bp5-021-L");
    let filler_member = game.id("PL!-sd1-007-SD"); // Printemps unit, not 蓮ノ空
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(filler_member);
    game.play_to_stage(filler_member, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // No eligible 蓮ノ空 member → no multiplier should be set at all
    assert!(
        game.state.mods.heart_color_multiplier.is_empty(),
        "heart_color_multiplier should be empty when no 蓮ノ空 members on stage"
    );
}

/// ab#1: 3+ みらくらぱーく！ members → score +1.
#[test]
fn hs_bp5_021_score_bonus_with_three_mirakura_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!HS-bp5-021-L");
    let mirakura = game.id("PL!HS-sd1-003-SD"); // みらくらぱーく！
    let filler = game.id("PL!-sd1-010-SD");

    // Place 3 みらくらぱーく！ members on stage (cost 7 each)
    game.give_energy(30);
    fill_decks(&mut game, filler);

    let mirakura_b = game.new_id("PL!HS-sd1-003-SD");
    let mirakura_c = game.new_id("PL!HS-sd1-003-SD");

    game.state.player1.hand.cards.push(mirakura);
    game.state.player1.hand.cards.push(mirakura_b);
    game.state.player1.hand.cards.push(mirakura_c);
    game.play_to_stage(mirakura, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(mirakura_b, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(mirakura_c, MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // ab#1: condition met → score +1
    let score_mod = game.state.mods.get_score_modifier(live_card);
    assert!(
        score_mod >= 1,
        "Score should be +1 with 3 みらくらぱーく！ members (got {})",
        score_mod
    );
}

/// ab#1: <3 みらくらぱーく！ members → no score bonus.
#[test]
fn hs_bp5_021_no_score_bonus_with_one_mirakura_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!HS-bp5-021-L");
    let mirakura = game.id("PL!HS-sd1-003-SD"); // みらくらぱーく！
    let other = game.id("PL!-sd1-010-SD"); // not みらくらぱーく！
    let filler = game.id("PL!-sd1-010-SD");

    // Place 1 みらくらぱーく！ + 1 other member on stage
    game.give_energy(20);
    fill_decks(&mut game, filler);

    game.state.player1.hand.cards.push(mirakura);
    game.state.player1.hand.cards.push(other);
    game.play_to_stage(mirakura, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.play_to_stage(other, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // ab#1: condition NOT met (only 1 みらくらぱーく！ member) → score +0
    let score_mod = game.state.mods.get_score_modifier(live_card);
    assert_eq!(
        score_mod, 0,
        "Score should be +0 with only 1 みらくらぱーく！ member (got {})",
        score_mod
    );
}
