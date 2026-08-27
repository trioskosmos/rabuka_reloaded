use crate::helpers::*;
use crate::test_modules::support::bp7_wait_immunity_helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Tests for previously untested edge-case abilities
// ====================================================================

/// 穂乃果's cost-limit wait is blocked by 松浦果南's wait-immunity on the target.
#[test]
fn honoka_cost_limit_wait_blocked_by_wait_immunity() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());

    // Player2 protects their 果南 (cost 4, blade 2).
    let p2_kanan = p2_establish_wait_immunity(&mut g);

    // Player1 activates 穂乃果 (self→discard, wait opponent cost ≤ 4).
    let honoka = g.id("PL!-bp6-010-N");
    g.state.player1.hand.cards.push(honoka);
    g.give_energy(10);
    g.play_to_stage(honoka, MemberArea::RightSide);
    g.activate_ability(honoka);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        !is_waited(&g, p2_kanan),
        "穂乃果's cost-limit wait must be blocked by wait-immunity"
    );
}

/// Test Named Baton Touch Ability (`PL!N-pb1-022-P+` - Mia Taylor)
/// "登場:「三船栞子」からバトンタッチして登場した場合、カードを2枚引き、手札を1枚控え室に置く。"
/// Verifies that the ability only triggers if the replaced card is precisely the specified member.
#[test]
fn test_named_baton_touch_mia() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Setup cards
    let mia = game.id("PL!N-pb1-022-P+"); // Target test card
    let shioriko = game.id("PL!N-sd1-010-SD"); // Valid baton touch target
    let kasumi = game.id("PL!N-sd1-002-SD"); // Invalid baton touch target
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15); // Plentiful energy to play cards (Mia costs ~11)

    // Scenario 1: Valid Baton Touch (Mia replaces Shioriko)
    game.add_to_stage(MemberArea::Center, shioriko);
    game.state.player1.hand.cards.push(mia);

    // Add dummy cards so we can test the "draw 2 discard 1"
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    let _hand_before = game.state.player1.hand.cards.len(); // Should be 1 (mia)

    // Play Mia over Shioriko
    game.play_to_stage(mia, MemberArea::Center);

    // Hand size: Started at 1 (Mia). Played Mia -> 0. Ability triggers: Draw 2 -> 2. Discard 1 -> 1.
    // If it's waiting for choice, we need to provide one.
    assert!(
        game.has_pending_choice(),
        "Mia should trigger and wait for discard choice after drawing 2"
    );

    // Hand should now have 2 cards (drawn from deck)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "Hand should have 2 drawn cards before discard"
    );

    // Discard the first card
    game.select_indices(&[0]);

    // Resolved! Hand should now be 1.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Hand should have 1 card after completing discard"
    );

    // Scenario 2: Invalid Baton Touch (Mia replaces Kasumi)
    let mia2 = game.new_id("PL!N-pb1-022-P+"); // A second copy of Mia

    // Clear center and put Kasumi there.
    // Also remove Center card from deployed_this_turn (simulating it wasn't deployed this turn).
    let center_card = game.state.player1.stage.stage[1];
    game.state.player1.stage.stage[1] = -1;
    game.state
        .player1
        .deployed_this_turn
        .retain(|id| *id != center_card);
    game.add_to_stage(MemberArea::Center, kasumi);
    game.state.player1.hand.cards.push(mia2);

    // Ensure deck has cards
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);

    // Play Mia over Kasumi
    game.play_to_stage(mia2, MemberArea::Center);

    // Ability should NOT trigger because Kasumi is not Shioriko. No pending choice!
    assert!(
        !game.has_pending_choice(),
        "Mia should NOT trigger because she did not replace Shioriko"
    );
}

/// Test Hand Cost Reduction (`LL-bp2-001-R＋` - Kotori Minami)
/// "常時: 手札にあるこのメンバーカードのコストは、このカード以外の自分の手札1枚につき、1少なくなる。"
/// Verifies that playing the card consumes the correctly reduced amount of energy.
#[test]
fn test_hand_size_cost_reduction_kotori() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let kotori = game.id("LL-bp2-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    let _base_cost = game.db.get_card(kotori).unwrap().cost.unwrap();

    // Scenario 1: Insufficient energy for reduced cost
    // Hand has 3 cards (Kotori + 2 filler). Other cards = 2.
    // Expected cost = 20 - 2 = 18.
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(kotori);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    game.state.player1.energy_zone.cards.clear();
    game.state.player1.energy_zone.set_active_count(0);
    game.give_energy(17); // 1 short of the expected cost of 18

    // Attempt to play Kotori. Should fail.
    let res = game.try_play_to_stage(kotori, MemberArea::Center);
    assert!(
        res.is_err(),
        "Should fail to play Kotori with only 17 energy (cost should be 18)"
    );

    // Scenario 2: Sufficient energy for reduced cost
    game.give_energy(1); // Now have 18 energy
    let res = game.try_play_to_stage(kotori, MemberArea::Center);
    assert!(
        res.is_ok(),
        "Should successfully play Kotori with 18 energy (base 20 - 2 reduction)"
    );

    // Verify energy consumed
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 18 energy should have been consumed"
    );

    // Scenario 3: Verify it doesn't reduce below 0 (though base is 20, let's verify logic)
    let mut game2 = TestGame::new(db.clone());
    game2.state.player1.hand.cards.clear();
    game2.state.player1.hand.cards.push(kotori);
    // Add 25 fillers. Reduction = 25. 20 - 25 = -5 -> 0.
    for _ in 0..25 {
        game2.state.player1.hand.cards.push(filler);
    }
    game2.state.player1.energy_zone.cards.clear();
    game2.state.player1.energy_zone.set_active_count(0);
    // No energy given. Cost should be 0.
    let res = game2.try_play_to_stage(kotori, MemberArea::Center);
    assert!(
        res.is_ok(),
        "Should play Kotori for 0 energy with 25 cards in hand"
    );
}

/// Test Opponent Choice Ability (`PL!N-PR-022-PR` - Emma Verde)
/// "登場: ...相手にエマパンチ打つ？と聞いてもよい。回答がお願いしますの場合、...相手のステージにいるすべてのメンバーは、ブレードを得る。"
/// Verifies that choice_player_id is set to opponent and effects apply correctly.
#[test]
fn test_opponent_choice_emma_punch() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let emma = game.id("PL!N-PR-022-PR");
    let filler = game.id("PL!-sd1-010-SD");

    // Setup: Player 2 has a card on stage
    game.state
        .player2
        .stage
        .set_area(MemberArea::Center, filler);

    // Player 1 plays Emma
    game.state.player1.hand.cards.push(emma);
    game.give_energy(10);
    game.play_to_stage(emma, MemberArea::LeftSide);

    // Ability should have triggered and paused for choice
    assert!(
        game.has_pending_choice(),
        "Emma's Debut ability should be waiting for choice"
    );

    // Verify choice_player_id is "p2" (opponent)
    let entry = game
        .state
        .ability_queue
        .current_entry()
        .expect("Queue should have an entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Choice player should be p2 (opponent)"
    );

    // Opponent chooses "Yes please" (index 0)
    game.select_option(0);

    // Verify Player 2's member on stage got a blade
    let p2_center_blade = game.state.mods.get_blade_modifier(filler);
    assert_eq!(
        p2_center_blade, 1,
        "Player 2's member should have gained 1 blade"
    );

    // Verify Player 1's Emma should NOT gain a blade (target was "opponent")
    let p1_emma_blade = game.state.mods.get_blade_modifier(emma);
    assert_eq!(
        p1_emma_blade, 0,
        "Player 1's Emma should NOT have gained a blade"
    );
}

/// Test Change State Choice Bug (`PL!-PR-007-PR` - Nozomi Tojo)
/// "登場: このメンバーをウェイトにしてもよい：相手のステージにいるコスト4以下のメンバー1人をウェイトにする。"
/// Verifies that chosen member remains on stage in wait state instead of being discarded.
#[test]
fn test_change_state_choice_bug() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let nozomi = game.id("PL!-PR-007-PR");
    let target1 = game.id("PL!-sd1-010-SD"); // Filler 1
    let target2 = game.id("PL!-sd1-011-SD"); // Filler 2

    // Setup: Player 2 has two members on stage
    game.state
        .player2
        .stage
        .set_area(MemberArea::Center, target1);
    game.state
        .player2
        .stage
        .set_area(MemberArea::LeftSide, target2);

    // Player 1 plays Nozomi
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(10);
    game.play_to_stage(nozomi, MemberArea::RightSide);

    // 1. Optional cost prompt (ChoiceMaker should be p1 by default)
    assert!(
        game.has_pending_choice(),
        "Nozomi should wait for optional cost choice"
    );
    game.select_option(1); // Choose "Yes" (Pay optional cost: Nozomi becomes wait)

    // Verify Nozomi is in wait state
    let nozomi_orientation = game.state.mods.get_orientation_modifier(nozomi);
    assert_eq!(
        nozomi_orientation,
        Some("wait"),
        "Nozomi should be in wait state"
    );

    // 2. Effect target prompt (Two valid targets on P2 stage)
    assert!(
        game.has_pending_choice(),
        "Nozomi should wait for effect target choice"
    );

    // Select target1 (Center)
    // In SelectCard choice for stage, indices are 0-2 (LeftSide, Center, RightSide)
    // target1 is at Center (index 1)
    game.select_indices(&[1]);

    // Verify target1 is STILL ON STAGE and in WAIT STATE
    let p2_center = game.state.player2.stage.get_area(MemberArea::Center);
    assert_eq!(
        p2_center,
        Some(target1),
        "Target member should still be on stage at Center"
    );

    let target1_orientation = game.state.mods.get_orientation_modifier(target1);
    assert_eq!(
        target1_orientation,
        Some("wait"),
        "Target member should be in wait state"
    );
}

/// Test Mirakura Park Baton Touch (`PL!HS-bp2-009-R` - Himeno)
/// "登場: Eを1枚支払ってもよい：このメンバーよりコストが低い『みらくらぱーく！』のメンバーからバトンタッチして登場した場合、
///  ライブ終了時まで、heart01 heart01を得る。"
/// Verifies group_names and cost comparison checks in movement_condition.
#[test]
fn test_mirakura_baton_touch_group_cost_check() {
    let db = load_real_database();
    use rabuka_engine::card::HeartColor;

    // ── Test 1: Baton touch from NON-みらくらぱーく！ member → condition fails ──
    {
        let mut g = TestGame::new(db.clone());
        let himeno = g.id("PL!HS-bp2-009-R");
        let non_mirakura = g.id("PL!-sd1-010-SD"); // unit=Printemps, not みらくらぱーく！

        g.state.player1.stage.stage[1] = non_mirakura;
        g.state.player1.hand.cards.push(himeno);
        g.give_energy(20);
        g.play_to_stage(himeno, MemberArea::Center);

        // Optional cost: pay 1 energy
        assert!(
            g.has_pending_choice(),
            "Test1: Should have optional cost prompt"
        );
        g.select_option(1); // Pay

        // No more choice — condition should fail: non_mirakura is not みらくらぱーく！
        assert!(
            !g.has_pending_choice(),
            "Test1: No further choice — condition should fail (wrong group)"
        );
        let heart01 = g.state.mods.get_heart_modifier(himeno, HeartColor::Heart01);
        assert_eq!(
            heart01, 0,
            "Test1: Should NOT gain heart01 — baton touch source is not みらくらぱーく！"
        );
    }

    // ── Test 2: Baton touch from lower-cost みらくらぱーく！ member → condition passes ──
    {
        let mut g = TestGame::new(db.clone());
        let himeno = g.id("PL!HS-bp2-009-R"); // cost=13
        let mirakura_low = g.id("PL!HS-sd1-011-SD"); // cost=4, みらくらぱーく！

        g.state.player1.stage.stage[1] = mirakura_low; // cost=4 < himeno's cost=13
        g.state.player1.hand.cards.push(himeno);
        g.give_energy(20);
        g.play_to_stage(himeno, MemberArea::Center);

        assert!(
            g.has_pending_choice(),
            "Test2: Should have optional cost prompt"
        );
        g.select_option(1); // Pay

        // Condition should pass: lower-cost みらくらぱーく！ → gain 2 heart01
        let heart01 = g
            .state
            .mods
            .get_heart_modifier(g.state.player1.stage.stage[1], HeartColor::Heart01);
        assert_eq!(
            heart01, 2,
            "Test2: Should gain 2 heart01 — correct group + lower cost"
        );
    }

    // ── Test 3: Baton touch from higher-cost みらくらぱーく！ member → condition fails (cost check) ──
    {
        let mut g = TestGame::new(db.clone());
        let himeno = g.id("PL!HS-bp2-009-R"); // cost=13
        let mirakura_high = g.id("PL!HS-bp2-006-R"); // cost=15, みらくらぱーく！

        g.state.player1.stage.stage[1] = mirakura_high; // cost=15 > himeno's cost=13
        g.state.player1.hand.cards.push(himeno);
        g.give_energy(20);
        g.play_to_stage(himeno, MemberArea::Center);

        assert!(
            g.has_pending_choice(),
            "Test3: Should have optional cost prompt"
        );
        g.select_option(1); // Pay

        // Condition should fail: higher-cost みらくらぱーく！ (cost check fails)
        let heart01 = g
            .state
            .mods
            .get_heart_modifier(g.state.player1.stage.stage[1], HeartColor::Heart01);
        assert_eq!(
            heart01, 0,
            "Test3: Should NOT gain heart01 — baton touch source has higher cost"
        );
    }
}

/// Advance helpers for live start (same pattern as sunny_test)
fn h_advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass(); // Main → Active
    game.pass(); // Active → Energy
    game.pass(); // Energy → Draw
    game.pass(); // Draw → Main
    game.pass(); // Main → LiveCardSetP1
}
fn h_advance_to_live_start(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → LiveStart
}

/// Test LiveStart per-unit heart counting from success_live_zone (`PL!-bp3-012-PR` - Kotori)
#[test]
fn test_success_live_zone_per_unit_heart() {
    let db = load_real_database();
    use rabuka_engine::card::HeartColor;

    // ── Empty success_live_zone → should gain 0 hearts ──
    {
        let mut g = TestGame::new(db.clone());
        let kotori = g.id("PL!-bp3-012-PR");
        let live = g.id("PL!-sd1-010-SD");

        // Fillers for phase draws
        g.state.player2.hand.cards.push(g.new_id("PL!-sd1-010-SD"));

        g.state.player1.stage.stage[1] = kotori;
        g.state.player1.hand.cards.push(live);
        g.give_energy(10);
        let filler = g.new_id("PL!-sd1-010-SD");
        for _ in 0..10 {
            g.state.player1.main_deck.cards.push(filler);
        }
        for _ in 0..10 {
            g.state.player2.main_deck.cards.push(filler);
        }

        h_advance_to_live_card_set_p1(&mut g);
        g.set_live_card(live);
        h_advance_to_live_start(&mut g);

        assert!(g.has_pending_choice(), "Should prompt for heart color");
        g.select_option(0); // Select heart01

        let heart01 = g.state.mods.get_heart_modifier(kotori, HeartColor::Heart01);
        assert_eq!(
            heart01, 0,
            "0 cards in success zone → 0 hearts (got {})",
            heart01
        );
    }

    // ── 3 cards in success_live_zone → should gain 3 hearts ──
    {
        let mut g = TestGame::new(db.clone());
        let kotori = g.id("PL!-bp3-012-PR");
        let live = g.id("PL!-sd1-010-SD");

        g.state.player2.hand.cards.push(g.new_id("PL!-sd1-010-SD"));

        g.state.player1.stage.stage[1] = kotori;
        g.state.player1.hand.cards.push(live);
        for _ in 0..3 {
            g.state
                .player1
                .success_live_card_zone
                .cards
                .push(g.new_id("PL!-sd1-010-SD"));
        }
        g.give_energy(10);
        let filler = g.new_id("PL!-sd1-010-SD");
        for _ in 0..10 {
            g.state.player1.main_deck.cards.push(filler);
        }
        for _ in 0..10 {
            g.state.player2.main_deck.cards.push(filler);
        }

        h_advance_to_live_card_set_p1(&mut g);
        g.set_live_card(live);
        h_advance_to_live_start(&mut g);

        assert!(
            g.has_pending_choice(),
            "Should prompt for heart color (3 cards)"
        );
        g.select_option(0); // Select heart01

        let heart01 = g.state.mods.get_heart_modifier(kotori, HeartColor::Heart01);
        assert_eq!(
            heart01, 3,
            "3 cards in success zone → 3 hearts (got {})",
            heart01
        );
    }
}

/// Test Honoka activation ability (`PL!-bp6-010-N`):
/// "起動: このメンバーをステージから控え室に置く：相手のステージにいるコスト4以下のメンバー1人をウェイトにする"
/// Verifies self_cost (move to waitroom) + change_state choice with filtered_indices.
#[test]
fn test_honoka_change_state_with_valid_targets() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());

    let honoka = g.id("PL!-bp6-010-N");
    let target1 = g.id("PL!-sd1-010-SD"); // cost 4
    let target2 = g.id("PL!-sd1-013-SD"); // cost 4

    // Opponent has 2 valid targets (cost ≤ 4)
    g.state.player2.stage.set_area(MemberArea::Center, target1);
    g.state
        .player2
        .stage
        .set_area(MemberArea::LeftSide, target2);

    // Player1 has Honoka on stage + enough energy
    g.state.player1.hand.cards.push(honoka);
    g.give_energy(10);
    g.play_to_stage(honoka, MemberArea::RightSide);

    // Activate Honoka's ability
    g.activate_ability(honoka);

    // Self-cost should have moved Honoka to waitroom
    let p1_right = g.state.player1.stage.get_area(MemberArea::RightSide);
    assert_eq!(p1_right, None, "Honoka should have left stage (self_cost)");
    assert!(
        g.state.player1.waitroom.cards.contains(&honoka),
        "Honoka should be in waitroom"
    );

    // Effect: change_state choice with 2 valid targets, count=1
    assert!(
        g.has_pending_choice(),
        "Should prompt for change_state target selection"
    );

    // Select target1 at Center (index 1: LeftSide=0, Center=1, RightSide=2)
    g.select_indices(&[1]);

    // Verify target1 is still on stage but in wait state
    let p2_center = g.state.player2.stage.get_area(MemberArea::Center);
    assert_eq!(p2_center, Some(target1), "Target should remain on stage");

    let target1_ori = g.state.mods.get_orientation_modifier(target1);
    assert_eq!(target1_ori, Some("wait"), "Target should be in wait state");

    // Verify target2 is unchanged
    let target2_ori = g.state.mods.get_orientation_modifier(target2);
    assert_eq!(
        target2_ori, None,
        "Unselected target should have no orientation modifier"
    );
}

/// Test Dia debut ability (`PL!S-bp5-004-R`) option 0 with valid Aqours targets:
/// "登場: 以下から1つを選ぶ。
///  ・自分のステージにいるこのメンバー以外の『Aqours』のメンバー1人は、ライブ終了時まで、ブレードを得る。"
/// Verifies gain_resource with target_count + filtered_indices creates a SelectCard choice.
#[test]
fn test_dia_gain_resource_choice() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());

    let dia = g.id("PL!S-bp5-004-R");
    let aqours1 = g.id("PL!S-sd1-002-SD"); // Riko, unit=GuiltyKiss, series=サンシャイン→Aqours
    let aqours2 = g.id("PL!S-sd1-003-SD"); // Kanan, unit=AZALEA, series=サンシャイン→Aqours

    // Put 2 Aqours members on player1 stage
    g.state.player1.stage.set_area(MemberArea::Center, aqours1);
    g.state
        .player1
        .stage
        .set_area(MemberArea::LeftSide, aqours2);

    // Put Dia in hand
    g.state.player1.hand.cards.push(dia);
    g.give_energy(5); // Dia costs 2

    // Play Dia to stage (triggers debut)
    g.play_to_stage(dia, MemberArea::RightSide);

    // Debut: choice with 2 options
    assert!(
        g.has_pending_choice(),
        "Dia debut should show choice with 2 options"
    );

    // Select option 0: Aqours blade
    g.select_option(0);

    // With 2 valid targets (aqours1, aqours2) and target_count=1,
    // should create a SelectCard sub-choice
    assert!(
        g.has_pending_choice(),
        "Should prompt for Aqours member selection"
    );

    // Select aqours1 at Center (index 1: LeftSide=0, Center=1, RightSide=2)
    g.select_indices(&[1]);

    // Verify aqours1 got blade modifier
    let aqours1_blade = g.state.mods.get_blade_modifier(aqours1);
    assert!(
        aqours1_blade > 0,
        "Selected Aqours member should have blade modifier (got {})",
        aqours1_blade
    );

    // Verify aqours2 did NOT get blade modifier
    let aqours2_blade = g.state.mods.get_blade_modifier(aqours2);
    assert_eq!(
        aqours2_blade, 0,
        "Unselected Aqours member should NOT have blade modifier"
    );
}

/// Test Dia debut ability option 0 with NO valid targets (only Dia herself on stage).
/// Should end silently without creating a further selection choice.
#[test]
fn test_dia_gain_resource_no_targets() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());

    let dia = g.id("PL!S-bp5-004-R");

    // Only Dia on stage (no other Aqours members)
    g.state.player1.hand.cards.push(dia);
    g.give_energy(5);

    g.play_to_stage(dia, MemberArea::Center);

    // Debut: choice with 2 options
    assert!(
        g.has_pending_choice(),
        "Dia debut should show choice with 2 options"
    );

    // Select option 0: Aqours blade
    g.select_option(0);

    // No valid targets (only Dia, excluded by exclude_self)
    // Should end silently without a further choice
    assert!(
        !g.has_pending_choice(),
        "Should NOT prompt for target selection when no valid targets"
    );

    // Verify Dia herself does NOT have blade modifier (excluded by exclude_self)
    let dia_blade = g.state.mods.get_blade_modifier(dia);
    assert_eq!(
        dia_blade, 0,
        "Dia should not receive blade (excluded by exclude_self)"
    );
}
