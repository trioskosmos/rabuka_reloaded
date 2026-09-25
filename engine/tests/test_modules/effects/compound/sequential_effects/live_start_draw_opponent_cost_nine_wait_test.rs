/// Tests for PL!N-bp4-004-R＋ 朝香果林 (Asaka Karin):
///
/// Ability #0 (ライブ開始時):
///   Draw 1 card. Change up to 1 opponent member with cost ≤ 9 to Wait.
///   max=true means highest-cost eligible member is chosen first.
///
/// Ability #1 (ライブ開始時):
///   Select 虹ヶ咲 member cards from discard equal to count of opponent's
///   Wait-state members. Place them on top of deck in any order.
///   This uses `dynamic_count` — the count is derived at runtime.
use crate::helpers::*;

const KARIN: &str = "PL!N-bp4-004-R\u{ff0b}";
const FILLER: &str = "PL!-sd1-010-SD";

fn trigger_live_start_all(game: &mut TestGame) {
    let pid = game.state.player1.id.clone();
    let stage_cards: Vec<i16> = game.state.player1.stage.stage.iter().copied().filter(|&c| c != -1).collect();
    for card_id in stage_cards {
        let card = match game.db.get_card(card_id) {
            Some(c) => c,
            None => continue,
        };
        for ab in card.resolved_abilities() {
            if ab.triggers.as_deref() == Some("ライブ開始時") {
                game.state.trigger_auto_ability(
                    format!("{}_{}", card.card_no, ab.full_text),
                    rabuka_engine::core::types::AbilityTrigger::LiveStart,
                    pid.clone(),
                    Some(card.card_no.to_string()),
                    Some(card_id),
                    None,
                    None,
                );
            }
        }
    }
    game.state.process_pending_auto_abilities(&pid);
}

fn resolve_all_choices(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        let choice = game.get_pending_choice();
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }
}

// ============================================================
// Ability #0: Draw 1 + Wait opponent's cost ≤9 member
// ============================================================

/// ab#0: opponent has cost≤9 member → wait applied, draw happens
#[test]
fn live_start_draw_cost_nine_wait_waits_eligible_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id(KARIN);
    let _filler = game.id(FILLER);
    let cost5_member = game.id("PL!-sd1-014-SD"); // cost 5

    game.state.player1.stage.stage = [karin, -1, -1];
    game.state.player2.stage.stage[0] = cost5_member;

    // Fill decks
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }

    let hand_before = game.state.player1.hand.cards.len();

    trigger_live_start_all(&mut game);
    resolve_all_choices(&mut game);

    // ab#0: should have drawn 1 card
    let hand_after = game.state.player1.hand.cards.len();
    assert!(
        hand_after >= hand_before + 1,
        "ab#0 should draw 1 card: before={}, after={}",
        hand_before,
        hand_after
    );

    // ab#0: cost5 member should be in Wait state
    let orient = game.state.mods.get_orientation_modifier(cost5_member);
    assert_eq!(
        orient,
        Some("wait"),
        "Cost-5 member should be Wait after ab#0"
    );
}

/// ab#0: opponent has cost>9 member only → no wait, just draw
#[test]
fn live_start_draw_cost_nine_wait_no_wait_over_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id(KARIN);
    let high_cost = game.id(KARIN); // cost 15 (Karin herself)

    game.state.player1.stage.stage = [karin, -1, -1];
    game.state.player2.stage.stage[0] = high_cost;

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }

    let hand_before = game.state.player1.hand.cards.len();

    trigger_live_start_all(&mut game);
    resolve_all_choices(&mut game);

    // Should draw but NOT wait the high-cost member
    let hand_after = game.state.player1.hand.cards.len();
    assert!(
        hand_after >= hand_before + 1,
        "Should still draw: before={}, after={}",
        hand_before,
        hand_after
    );

    let orient = game.state.mods.get_orientation_modifier(high_cost);
    assert_ne!(
        orient,
        Some("wait"),
        "Cost-15 member should NOT be waited"
    );
}

/// ab#0: empty opponent stage → just draw, no wait
#[test]
fn live_start_draw_cost_nine_wait_empty_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let karin = game.id(KARIN);

    game.state.player1.stage.stage = [karin, -1, -1];

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    let hand_before = game.state.player1.hand.cards.len();

    trigger_live_start_all(&mut game);
    resolve_all_choices(&mut game);

    let hand_after = game.state.player1.hand.cards.len();
    assert!(
        hand_after >= hand_before + 1,
        "Should draw: before={}, after={}",
        hand_before,
        hand_after
    );
}
