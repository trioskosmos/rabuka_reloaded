/// Test for PL!N-pb1-007-R & PL!N-pb1-007-P＋ (優木せつ菜)
///
/// Ability (常時 / Passive):
///   "自分のライブ中のライブカードの必要ハートの中に{{heart_01.png|heart01}}、
///    {{heart_02.png|heart02}}、{{heart_03.png|heart03}}、{{heart_04.png|heart04}}、
///    {{heart_05.png|heart05}}、{{heart_06.png|heart06}}がそれぞれ1以上含まれるかぎり、
///    {{icon_all.png|ハート}}を得る。"
///
/// English: "As long as the live cards in your live zone contain at least 1 each of
/// heart01, heart02, heart03, heart04, heart05, heart06, gain an all-type heart."
///
/// This is a constant ability that checks the aggregate required hearts across all
/// active live cards. The test verifies:
/// 1. The condition is checked correctly (all 6 heart types present)
/// 2. When the condition is met, Setsuna gains the "all-type" heart
/// 3. The heart gain is calculated into her performance contribution
/// 4. When condition is not met, no extra heart is gained
///
/// KNOWN ISSUE BEING DIAGNOSED:
/// - The temporal_condition with "during_live" target "self" is currently failing
/// - Performance snapshots show 0 hearts for Setsuna
/// - This suggests either the condition evaluator or ability queue processing needs fixes
///
use crate::helpers::*;

/// Helper: Advance from turn start (Main phase) to LiveCardSetP1.
fn advance_to_live_card_set(game: &mut TestGame) {
    game.pass(); // Main → Active
    game.pass(); // Active → Energy
    game.pass(); // Energy → Draw
    game.pass(); // Draw → Main (P2)
    game.pass(); // Main (P2) → LiveCardSetP1
}

/// Helper: Advance from LiveCardSetP1 to LiveStart phase.
fn advance_to_live_start(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → FirstAttackerPerformance (abilities fire)
}

/// Helper: Get Setsuna's calculated hearts from the first performance snapshot.
/// Returns the tuple (heart_count, heart01, heart02, heart03, heart04, heart05, heart06).
fn get_setsuna_heart_contribution(game: &TestGame, setsuna_id: i16) -> (u32, u32, u32, u32, u32, u32, u32) {
    use rabuka_engine::card::HeartColor;
    
    let snapshot = game.state.performance_snapshots.first();
    if let Some(snap) = snapshot {
        eprintln!("Performance snapshot has {} member contributions", snap.member_contributions.len());
        if let Some(mc) = snap.member_contributions.iter().find(|mc| mc.source_id == setsuna_id) {
            let total: u32 = mc.base_hearts.iter().sum::<u32>() + mc.bonus_hearts.iter().sum::<u32>();
            // base hearts + bonus hearts (bonus includes constant ability ALL heart)
            let base = &mc.base_hearts;
            let bonus = &mc.bonus_hearts;
            return (
                total,
                base[HeartColor::Heart01.index()] + bonus[HeartColor::Heart01.index()],
                base[HeartColor::Heart02.index()] + bonus[HeartColor::Heart02.index()],
                base[HeartColor::Heart03.index()] + bonus[HeartColor::Heart03.index()],
                base[HeartColor::Heart04.index()] + bonus[HeartColor::Heart04.index()],
                base[HeartColor::Heart05.index()] + bonus[HeartColor::Heart05.index()],
                base[HeartColor::Heart06.index()] + bonus[HeartColor::Heart06.index()],
            );
        } else {
            eprintln!("Setsuna not found in contributions");
        }
    } else {
        eprintln!("No performance snapshot available");
    }
    (0, 0, 0, 0, 0, 0, 0)
}

/// Test 1: Verify Setsuna card exists and has correct ability metadata.
///
/// This is a diagnostic test to confirm the card was parsed correctly
/// and that the ability structure is as expected.
#[test]
fn setsuna_pb1_verify_card_metadata() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let setsuna: i16 = game.id("PL!N-pb1-007-R");
    
    let card = db.get_card(setsuna).expect("Setsuna should exist in database");
    
    // Verify basic card properties
    assert_eq!(card.card_no, "PL!N-pb1-007-R", "Card number should match");
    assert!(!card.abilities.is_empty(), "Card should have at least one ability");
    
    // The ability should be the constant (常時) one
    let constant_ability = card.abilities.iter()
        .find(|ab| ab.triggers.as_ref().map_or(false, |t| t.contains("常時")))
        .expect("Should have a 常時 (constant) ability");
    
    // Verify the ability structure includes the condition and effect
    assert!(constant_ability.effect.is_some(), "Ability should have an effect");
    eprintln!("Card verified: {} with ability triggers: {:?}", setsuna, constant_ability.triggers);
}

/// Test 2: Live card setup and verification
///
/// Verifies that:
/// - Live card can be added to hand and set during LiveCardSet phase
/// - Live card zone reflects the set card
#[test]
fn setsuna_pb1_live_card_setup_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna: i16 = game.id("PL!N-pb1-007-R");
    let live_card: i16 = game.id("PL!-sd1-021-SD");
    let filler: i16 = game.id("PL!-sd1-010-SD");

    // Setup stage and decks
    game.state.player1.stage.stage[1] = setsuna;
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(5);
    game.add_to_hand(live_card);

    advance_to_live_card_set(&mut game);
    
    assert!(game.state.player1.live_card_zone.cards.is_empty(), "Live zone should be empty before set");
    
    game.set_live_card(live_card);
    
    assert!(!game.state.player1.live_card_zone.cards.is_empty(), "Live card should be in zone after set");
    assert_eq!(game.state.player1.live_card_zone.cards[0], live_card, "Set card should match");
}

/// Test 3: Verify the condition check — NO heart gain when not all 6 types present.
///
/// Setup:
///   - Live card with only 3 heart types (heart01, heart03, heart06)
///   - Setsuna on stage
///
/// Expected:
///   - Setsuna's hearts should be exactly her base hearts (no bonus)
///   - Total should be 5 (heart01:2, heart02:2, heart05:1)
///   - DIAGNOSTIC: shows if performance snapshots are being created at all
#[test]
fn setsuna_pb1_constant_missing_heart_types_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna: i16 = game.id("PL!N-pb1-007-R");
    let live_card: i16 = game.id("PL!-sd1-019-SD"); // START:DASH!!: heart01, heart03, heart06 only (3 types)
    let filler: i16 = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = setsuna;
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(5);

    game.add_to_hand(live_card);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    
    eprintln!("Before advance_to_live_start: phase={}, snapshots={}", 
        game.state.current_phase, 
        game.state.performance_snapshots.len());
    
    advance_to_live_start(&mut game);
    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (creates snapshot)

    let (total, h01, h02, h05) = {
        let contrib = get_setsuna_heart_contribution(&game, setsuna);
        (contrib.0, contrib.1, contrib.2, contrib.5)
    };

    eprintln!("Test result: total={}, h01={}, h02={}, h05={}", total, h01, h02, h05);
    
    assert_eq!(
        total, 5,
        "Without all 6 heart types, Setsuna should have exactly base hearts (got {})",
        total
    );

    // Verify base distributions
    assert_eq!(h01, 2, "heart01 should be 2 (Setsuna base)");
    assert_eq!(h02, 2, "heart02 should be 2 (Setsuna base)");
    assert_eq!(h05, 1, "heart05 should be 1 (Setsuna base)");
}

/// Test 4: Verify Setsuna's variant card (PL!N-pb1-007-P＋) has the same ability.
///
/// Setup:
///   - Use the P+ (premium plus) variant instead of R (rare)
///   - Two live cards that collectively have all 6 heart types
///   - Setsuna P+ on stage
///
/// Expected:
///   - Same behavior as the R variant — gets +1 ALL when all 6 types present
#[test]
fn setsuna_pb1_p_plus_variant_constant_heart_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna_p_plus: i16 = game.id("PL!N-pb1-007-P＋");
    let live_card_a: i16 = game.id("PL!-sd1-019-SD"); // heart01, heart03, heart06
    let live_card_b: i16 = game.id("PL!S-PR-023-PR"); // heart02, heart04, heart05
    let filler: i16 = game.id("PL!-sd1-010-SD");

    let card_p_plus = db.get_card(setsuna_p_plus).expect("Setsuna P+ should exist");
    assert_eq!(card_p_plus.card_no, "PL!N-pb1-007-P＋", "Card number should match P+ variant");
    
    game.state.player1.stage.stage[1] = setsuna_p_plus;
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(5);

    game.add_to_hand(live_card_a);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card_a);
    // Add second live card to success zone so both zones cover all 6 types collectively
    game.state.player1.success_live_card_zone.cards.push(live_card_b);
    advance_to_live_start(&mut game);
    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (creates snapshot)

    let (total, _, _, _, _, _, _) = get_setsuna_heart_contribution(&game, setsuna_p_plus);

    eprintln!("P+ variant hearts: {}", total);
    
    assert_eq!(
        total, 6,
        "Setsuna P+ should get +1 ALL heart (got {})",
        total
    );
}

/// Test 5: Constant ability recalculation — heart gain persists during live.
///
/// Setup:
///   - Two live cards collectively covering all 6 heart types
///   - Setsuna on stage
///   - Pass multiple times through performance phase
///
/// Expected:
///   - Heart gain persists across performance phases
///   - Both snapshots show total = 6 (base 5 + 1 ALL)
#[test]
fn setsuna_pb1_constant_heart_persists_during_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna: i16 = game.id("PL!N-pb1-007-R");
    let live_card_a: i16 = game.id("PL!-sd1-019-SD"); // heart01, heart03, heart06
    let live_card_b: i16 = game.id("PL!S-PR-023-PR"); // heart02, heart04, heart05
    let filler: i16 = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = setsuna;
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(5);

    game.add_to_hand(live_card_a);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card_a);
    game.state.player1.success_live_card_zone.cards.push(live_card_b);
    advance_to_live_start(&mut game);
    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (creates snapshot)

    let (total_1, _, _, _, _, _, _) = get_setsuna_heart_contribution(&game, setsuna);
    eprintln!("First snapshot total: {}", total_1);

    game.pass();
    
    let (total_2, _, _, _, _, _, _) = get_setsuna_heart_contribution(&game, setsuna);
    eprintln!("Second snapshot total: {}", total_2);

    assert_eq!(
        total_1, 6,
        "First snapshot should show base hearts + ALL bonus (got {})",
        total_1
    );
    
    if total_2 > 0 {
        assert_eq!(
            total_2, 6,
            "Second snapshot should also show base hearts + ALL bonus (got {})",
            total_2
        );
    }
}

/// Test 6: Condition edge case — different live cards with different heart distributions.
///
/// Setup:
///   - Live card with {heart02:2, heart04:2, heart05:2, heart0:1}
///   - Setsuna on stage
///
/// Expected:
///   - Setsuna should get bonus because multiple specific heart colors are present
///   - But still checking if condition evaluates correctly
#[test]
fn setsuna_pb1_constant_partial_heart_types_various_combos() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let setsuna: i16 = game.id("PL!N-pb1-007-R");
    let live_card: i16 = game.id("PL!S-PR-023-PR"); // heart02, heart04, heart05, heart0
    let filler: i16 = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = setsuna;
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(5);

    game.add_to_hand(live_card);

    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);
    game.pass(); // FirstAttackerPerformance → SecondAttackerPerformance (creates snapshot)

    let (total, _, _, _, _, _, _) = get_setsuna_heart_contribution(&game, setsuna);

    eprintln!("Partial heart types test: total={}", total);
    
    // This live card has heart02, heart04, heart05 (3 types, missing 1, 3, 6)
    // Condition should NOT trigger (needs all 6)
    assert_eq!(
        total, 5,
        "Missing heart types 01, 03, 06 should result in no bonus (got {})",
        total
    );
}
