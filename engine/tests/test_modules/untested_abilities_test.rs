use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ====================================================================
// Tests for previously untested edge-case abilities
// ====================================================================

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

    game.give_energy(10); // Plentiful energy to play cards

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
    // Wait, the ability triggers and we have to select a card to discard.
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
    
    // Clear center and put Kasumi there
    game.state.player1.stage.stage[1] = -1;
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

    let base_cost = game.db.get_card(kotori).unwrap().cost.unwrap();

    // Scenario 1: Insufficient energy for reduced cost
    // Hand has 3 cards (Kotori + 2 filler). Other cards = 2.
    // Expected cost = 20 - 2 = 18.
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(kotori);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    
    game.state.player1.energy_zone.cards.clear();
    game.state.player1.energy_zone.active_energy_count = 0;
    game.give_energy(17); // 1 short of the expected cost of 18

    // Attempt to play Kotori. Should fail.
    let res = game.try_play_to_stage(kotori, MemberArea::Center);
    assert!(res.is_err(), "Should fail to play Kotori with only 17 energy (cost should be 18)");

    // Scenario 2: Sufficient energy for reduced cost
    game.give_energy(1); // Now have 18 energy
    let res = game.try_play_to_stage(kotori, MemberArea::Center);
    assert!(res.is_ok(), "Should successfully play Kotori with 18 energy (base 20 - 2 reduction)");

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
    game2.state.player1.energy_zone.active_energy_count = 0;
    // No energy given. Cost should be 0.
    let res = game2.try_play_to_stage(kotori, MemberArea::Center);
    assert!(res.is_ok(), "Should play Kotori for 0 energy with 25 cards in hand");
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
    game.state.player2.stage.set_area(MemberArea::Center, filler);
    
    // Player 1 plays Emma
    game.state.player1.hand.cards.push(emma);
    game.give_energy(10);
    game.play_to_stage(emma, MemberArea::LeftSide);

    // Ability should have triggered and paused for choice
    assert!(game.has_pending_choice(), "Emma's Debut ability should be waiting for choice");
    
    // Verify choice_player_id is "p2" (opponent)
    let entry = game.state.ability_queue.current_entry().expect("Queue should have an entry");
    assert_eq!(
        entry.choice_player_id.as_deref(),
        Some("p2"),
        "Choice player should be p2 (opponent)"
    );

    // Opponent chooses "Yes please" (index 0)
    game.select_option(0);

    // Verify Player 2's member on stage got a blade
    let p2_center_blade = game.state.mods.get_blade_modifier(filler);
    assert_eq!(p2_center_blade, 1, "Player 2's member should have gained 1 blade");

    // Verify Player 1's Emma should NOT gain a blade (target was "opponent")
    let p1_emma_blade = game.state.mods.get_blade_modifier(emma);
    assert_eq!(p1_emma_blade, 0, "Player 1's Emma should NOT have gained a blade");
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
    game.state.player2.stage.set_area(MemberArea::Center, target1);
    game.state.player2.stage.set_area(MemberArea::LeftSide, target2);
    
    // Player 1 plays Nozomi
    game.state.player1.hand.cards.push(nozomi);
    game.give_energy(10);
    game.play_to_stage(nozomi, MemberArea::RightSide);

    // 1. Optional cost prompt (ChoiceMaker should be p1 by default)
    assert!(game.has_pending_choice(), "Nozomi should wait for optional cost choice");
    game.select_option(1); // Choose "Yes" (Pay optional cost: Nozomi becomes wait)

    // Verify Nozomi is in wait state
    let nozomi_orientation = game.state.mods.get_orientation_modifier(nozomi);
    assert_eq!(nozomi_orientation.map(|s| s.as_str()), Some("wait"), "Nozomi should be in wait state");

    // 2. Effect target prompt (Two valid targets on P2 stage)
    assert!(game.has_pending_choice(), "Nozomi should wait for effect target choice");
    
    // Select target1 (Center)
    // In SelectCard choice for stage, indices are 0-2 (LeftSide, Center, RightSide)
    // target1 is at Center (index 1)
    game.select_indices(&[1]); 

    // Verify target1 is STILL ON STAGE and in WAIT STATE
    let p2_center = game.state.player2.stage.get_area(MemberArea::Center);
    assert_eq!(p2_center, Some(target1), "Target member should still be on stage at Center");
    
    let target1_orientation = game.state.mods.get_orientation_modifier(target1);
    assert_eq!(target1_orientation.map(|s| s.as_str()), Some("wait"), "Target member should be in wait state");
}
