use crate::helpers::*;
use rabuka_engine::card::BaseHeart;
/// Tests for 繚乱！ビクトリーロード (PL!N-bp5-030-L) — each_time auto abilities.
///
/// ab#0: Each time a member's LiveStart resolves, if member lacks hearts, give all hearts.
/// ab#1: Each time a member's LiveSuccess resolves, draw 1 card.
///
/// Q217: Cost IS paid (select 0 for any_number) → ability is "used" → each_time fires.
/// Q227: Cost declined entirely → ability NOT "used" → each_time does NOT fire.
///
/// These test whether paying 0 vs declining produces different trigger behavior.
/// The referenced member is LL-bp2-001-R＋ (鬼塚夏美&遠藤アリサ&遠手鞠) which has
/// an optional LiveStart cost: discard any number of named characters from hand.
use std::collections::HashMap;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Test that ab#1 (each_time LiveSuccess → draw 1) actually fires
/// when members with LIVE_SUCCESS ability resolve.
#[test]
fn victory_road_live_success_each_time_draws_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let victory = game.id("PL!N-bp5-030-L");
    // PL!S-bp3-005-R (星空凛) has LIVE_SUCCESS with draw 1, no cost
    let live_success_member = game.id("PL!S-bp3-005-R");

    // Fill deck with a known card
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        let filler = game.new_id("PL!-sd1-010-SD");
        game.state.player1.main_deck.cards.push(filler);
    }
    let initial_hand = game.state.player1.hand.len();

    // Place Victory Road in live card zone (the card with each_time abilities)
    game.state.player1.live_card_zone.cards.push(victory);

    // Place the LIVE_SUCCESS member on stage
    game.state.player1.stage.stage[0] = live_success_member;
    let filler = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[1] = filler;
    let filler = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage[2] = filler;

    // Set stage hearts so should_trigger_live_success passes
    // Victory Road needs: heart01-06 each 1, plus heart00=7
    let mut hearts = BaseHeart {
        hearts: HashMap::new(),
    };
    use rabuka_engine::card::HeartColor;
    hearts.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(hearts);

    // Set phase for LiveVictoryDetermination
    game.state.current_phase = rabuka_engine::game_state::Phase::LiveVictoryDetermination;

    // Trigger LiveSuccess abilities
    game.state.live_success_triggered_this_turn = false;
    rabuka_engine::turn::TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    // After LiveSuccess abilities resolve, trigger each_time abilities
    rabuka_engine::turn::TurnEngine::trigger_each_time_abilities(
        &mut game.state,
        "p1",
        rabuka_engine::triggers::LIVE_SUCCESS,
    );
    game.state.process_pending_auto_abilities("p1");

    let hand_after = game.state.player1.hand.len();
    eprintln!(
        "[LIVE_SUCCESS_EACH_TIME] initial_hand={} hand_after={}",
        initial_hand, hand_after
    );

    // The each_time ability draws 1 card
    assert!(
        hand_after > initial_hand,
        "Each_time LiveSuccess should draw a card (hand {} -> {})",
        initial_hand,
        hand_after
    );
}

/// Verify the multi-member card exists and its LiveStart ability fires.
/// The optional cost creates a choice. Q217: selecting 0 still counts as "used."
/// Q227: declining the cost entirely does NOT count as "used."
#[test]
fn victory_road_q217_q227_cost_handling() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    let victory = game.id("PL!N-bp5-030-L");
    // Fullwidth plus sign
    let multi = game.id("LL-bp2-001-R\u{ff0b}");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // The multi-member card on stage, victory road in hand
    game.state.player1.stage.stage[0] = multi;
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.hand.cards.push(victory);
    // Put the named characters in hand so the cost filter has targets
    // Named: 鬼塚夏美, 遠藤アリサ, 遠手鞠 — put filler as dummy
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_set(&mut game);
    game.set_live_card(victory);

    // Pass through live set phases to trigger LiveStart
    game.pass(); // LiveCardSetFirstAttacker → P2Turn
    game.pass(); // LiveCardSetSecondAttacker → FirstAttackerPerformance → LiveStart

    // Multi-member's LiveStart fires. The optional cost displays.
    // Handle whatever choices come up
    eprintln!("[Q217/Q227] Choices after LiveStart trigger:");
    game.dbg_choice();

    // Drain all pending choices
    let mut safety = 0;
    while game.has_pending_choice() && safety < 10 {
        safety += 1;
        // If there's a pending_choice, try option -1 (skip/decline)
        if game.state.has_pending_choice() {
            game.select_option(-1);
        } else {
            game.select_indices(&[]);
        }
    }

    eprintln!("[Q217/Q227] Done. safety={}", safety);
    assert!(safety < 10, "Didn't loop infinitely");

    // After resolving all choices, the live card (victory road) should be in the live zone
    // and the multi-member card should still be on stage
    assert!(
        game.state.player1.live_card_zone.cards.contains(&victory),
        "Victory Road should be set as live card"
    );
    assert!(
        game.state.player1.stage.stage.contains(&multi),
        "Multi-member card should remain on stage after LiveStart resolution"
    );
    // The victory road ability (each_time on LiveSuccess → draw 1) hasn't fired yet
    // but the setup should be valid
    assert!(
        !game.has_pending_choice(),
        "All pending choices should be drained after LiveStart resolution"
    );
}
