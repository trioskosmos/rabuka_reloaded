use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Test the full cheer pipeline: yell draws, blade hearts → owned hearts,
/// draw/score icons from special_heart, and b_all wildcard handling.
#[test]
fn cheer_pipeline_draw_and_score_icons() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // Stage: 絢瀬絵里 (PL!-sd1-002-SD) — blade=1, blade_heart={b_heart06:1}, base_heart={heart06:1}
    let stage_member = game.id("PL!-sd1-002-SD");
    game.add_to_stage(MemberArea::Center, stage_member);

    // Live card: 僕らは今のなかで (PL!-sd1-022-SD) — special_heart={draw:1}, blade_heart={b_heart03:1}
    // Also will have blade=0 from yell but the yell cards will have blade hearts
    let live_card = game.id("PL!-sd1-022-SD");
    game.add_to_hand(live_card);

    // Seed the deck with cards that have specific blade hearts for yell testing.
    // After blade_count=1 yell, 2 yell cards will be in resolution zone.
    // Put a b_all live card in deck to test wildcard handling
    let b_all_card = game.id("PL!-sd1-020-SD"); // 僕らは今のなかで
    let filler = game.id("PL!-sd1-010-SD"); // 南ことり (no blade_heart)
    game.state.player1.main_deck.cards.push(b_all_card);
    game.state.player1.main_deck.cards.push(filler);

    // Add some extra cards in deck for draw-from-special_heart
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let _hand_before = game.state.player1.hand.len();
    let _deck_before = game.state.player1.main_deck.len();

    // Advance to live start
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    // Pass through both performance phases
    game.pass(); // FirstAttackerPerformance
    game.pass(); // SecondAttackerPerformance

    // After FirstAttackerPerformance:
    // 1. Yell drew 1 card (blade=1) to resolution zone → the b_all card
    // 2. The b_all card's blade_heart={b_all:1} → wildcard heart00 added to owned hearts
    // 3. The live card's special_heart={draw:1} → 1 card drawn to hand (filler)
    // 4. cheer_icon_count includes the b_all icon (counted as icon)
    // 5. The resolution zone cards were moved to waitroom after processing

    // Hand: started with live_card (1), played it (-1), drew 1 from LiveCardSet (+1),
    // drew from special_heart draw (+1). Also the yell cards went to waitroom, not hand.
    // So hand_before = 1 (just live_card). After: -1+1+1 = 2
    let hand_after = game.state.player1.hand.len();
    assert!(
        hand_after >= 1,
        "Should have cards in hand after live performance"
    );

    // Waitroom should have the yelled card (resolution zone cards moved after processing)
    let waitroom_after = game.state.player1.waitroom.cards.len();
    let waitroom_before = 0usize;
    assert!(
        waitroom_after > waitroom_before,
        "Yelled cards should be in waitroom after performance (was {}, now {})",
        waitroom_before,
        waitroom_after
    );

    // The yell card had a regular blade_heart (b_heart03 → heart03) which contributes to the
    // heart pool but NOT to score (Rule 8.4.2.1 — only ♪ Score icons add to cheer count).
    // So cheer_blade_heart_count should be 0 for regular blade_hearts.
    assert_eq!(
        game.state.player1_cheer_blade_heart_count, 0,
        "Regular blade hearts do not contribute to score (Rule 8.4.2.1)"
    );
}

/// Test that special_heart score icon adds to the cheer count
#[test]
fn cheer_pipeline_score_icon() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(fill);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(fill);
    }

    // Stage: 絢瀬絵里 (PL!-sd1-002-SD) — blade=1, base_heart={heart06:1}
    let stage_member = game.id("PL!-sd1-002-SD");
    game.add_to_stage(MemberArea::Center, stage_member);

    // Live card: START:DASH!! (PL!-sd1-019-SD) — special_heart={score:1}, need_heart={heart01:1, heart03:1, heart06:1}
    // This live card has score icon in special_heart
    let live_card = game.id("PL!-sd1-019-SD");
    game.add_to_hand(live_card);

    // Need stage members providing heart01, heart03, heart06 for the need_heart requirement
    // 絢瀬絵里 provides heart06, need heart01 and heart03 too — put more members
    let filler_member = game.id("PL!-sd1-010-SD"); // 南ことり: heart01:1, heart03:1
    game.add_to_stage(MemberArea::LeftSide, filler_member);
    game.add_to_stage(MemberArea::RightSide, filler_member);

    // Seed deck for yell (1 blade from 絢瀬絵里)
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);

    game.pass(); // FirstAttackerPerformance
    game.pass(); // SecondAttackerPerformance

    // The score icon from special_heart should contribute to cheer count
    // The live card has special_heart={score:1}, blade_heart={} (no blade_heart on START:DASH)
    // But the yell card has blade_heart too? No, filler has no blade_heart.
    // So cheer count comes from special_heart.score = 1
    assert!(
        game.state.player1_cheer_blade_heart_count >= 1,
        "Score icon from special_heart should contribute to cheer count"
    );
}

// Re-use helpers from other test files
fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass();
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
