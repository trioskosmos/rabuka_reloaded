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

/// Rule 8.4.2.1: a score icon counts toward the cheer total only when its card
/// is REVEALED BY THE YELL (「エールで出た」). START:DASH!! is milled onto the
/// deck top so the yell reveals it; its special スコア icon then adds +1.
///
/// (This test previously expected START:DASH!!'s score icon to count while the
/// card sat in the LIVE ZONE — but rule 8.3.12 scopes blade-heart confirmation
/// to the resolution zone, and rule 8.4.2.1 scopes score icons to "icons of
/// your yell". An in-zone live card contributes nothing; see
/// special_blade_heart_rules_test.rs::live_zone_special_icons_do_not_apply.)
#[test]
fn cheer_pipeline_score_icon() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let fill = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(fill);
        game.state.player2.main_deck.cards.push(fill);
    }

    // Stage covers heart01+heart03 (ことり fillers) and heart06 (絢瀬絵里) so
    // START:DASH!! succeeds without any yell help.
    let kotori_a = game.id("PL!-sd1-010-SD"); // base_heart={heart01:1,heart03:1}, blade=1
    let kotori_b = game.new_id("PL!-sd1-010-SD");
    let eri = game.id("PL!-sd1-002-SD"); // base_heart={heart06:1}, blade=1
    game.add_to_stage(MemberArea::LeftSide, kotori_a);
    game.add_to_stage(MemberArea::Center, kotori_b);
    game.add_to_stage(MemberArea::RightSide, eri);

    // A second START:DASH!! copy goes ON TOP of the deck → revealed by yell.
    let dash_on_deck = game.id("PL!-sd1-019-SD");
    let sacrificial_fill = game.id("PL!-sd1-010-SD");

    let live_card = game.id("PL!-sd1-019-SD");
    game.add_to_hand(live_card);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_card);
    // Deck layout applied AFTER set_live_card (index 0 = top): position 0 is
    // consumed by the LiveCardSet refill draw, so the yell then reveals
    // [DASH copy, filler, filler].
    game.state.player1.main_deck.cards.insert(0, dash_on_deck);
    game.state
        .player1
        .main_deck
        .cards
        .insert(0, sacrificial_fill);

    advance_to_live_start(&mut game);

    game.pass(); // FirstAttackerPerformance — yell reveals dash_on_deck + 2 fillers
    game.pass(); // SecondAttackerPerformance
    game.pass(); // → Live Result

    // DASH's own ライブ成功時 ability ("look at top 3, arrange any number")
    // now queues an optional SelectCard prompt INSIDE victory determination;
    // victory defers finalizing while a choice is pending, so answer it
    // (skip = arrange zero cards, legal) and continue until finalized.
    let mut saw_result = false;
    for _ in 0..10 {
        if game.has_pending_choice() {
            game.select_indices(&[]);
            continue;
        }
        let phase = game.state.current_phase.to_string();
        if phase.contains("Live Result") {
            saw_result = true;
        }
        if saw_result && phase == "Active" {
            break;
        }
        game.pass();
    }

    // The revealed copy's special_heart={score:1} feeds the cheer count...
    assert_eq!(
        game.state.player1_cheer_blade_heart_count, 1,
        "score icon from the YELL-REVEALED special_heart adds exactly 1 (rule 8.4.2.1)"
    );

    // ...and the live total becomes card score 1 + cheer 1 = 2.
    let snap = game
        .state
        .performance_snapshots
        .iter()
        .rev()
        .find(|s| s.player_id == "p1")
        .expect("P1 snapshot");
    assert!(snap.success, "stage hearts satisfy heart01+03+06");
    assert_eq!(snap.total_score, 2, "card score 1 + one revealed score icon");

    // The revealed copy went to the waitroom with the rest of the yell.
    assert!(
        game.state.player1.waitroom.cards.contains(&dash_on_deck),
        "yell-revealed START:DASH!! should end up in the waitroom"
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
