/// Tests for JIMO-AI Dash! (PL!S-sd1-020-SD) ab#0 — LiveSuccess:
///   自分のステージにいる『Aqours』のメンバー1人につき、カードを1枚引く。
///   その後、これにより引いた枚数と同じ枚数を手札から控え室に置く。
///
/// Tests that the second action ("discard same number as drawn") uses the
/// correct dynamic_count resolved from last_draw_count.
use crate::helpers::*;

fn trigger_live_success(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .cloned()
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        rabuka_engine::core::types::AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

fn drain_choices(game: &mut TestGame) {
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                let count = game.pending_choice_count();
                if count == 0 {
                    // count=0 means nothing to select; skip by selecting nothing.
                    if let Err(_) = game.try_select_indices(&[]) {
                        break;
                    }
                } else {
                    // Select the first `count` available indices.
                    let indices: Vec<usize> = (0..count).collect();
                    game.select_indices(&indices);
                }
            }
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

fn setup(extra_hand: usize, aqours_stage: usize) -> (TestGame, i16, usize, usize) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sd20 = game.id("PL!S-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let aq_member = game.id("PL!S-sd1-001-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let mut stage = [-1; 3];
    for i in 0..aqours_stage.min(3) {
        stage[i] = if i == 0 {
            aq_member
        } else {
            game.new_id("PL!S-sd1-001-SD")
        };
    }
    game.state.player1.stage.stage = stage;

    for _ in 0..extra_hand {
        game.state.player1.hand.cards.push(filler);
    }

    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    trigger_live_success(&mut game, sd20);
    drain_choices(&mut game);

    (game, sd20, deck_before, hand_before)
}

/// 2 Aqours members on stage → draw 2 → discard 2 from hand.
/// Net hand size unchanged, deck reduced by 2.
#[test]
fn jimo_ai_draw_2_discard_2() {
    let (game, _, deck_before, hand_before) = setup(5, 2);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 2);
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
}

/// 1 Aqours member → draw 1 → discard 1.
#[test]
fn jimo_ai_draw_1_discard_1() {
    let (game, _, deck_before, hand_before) = setup(5, 1);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 1);
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
}

/// 0 Aqours members → draw 0 → no discard. Hand and deck unchanged.
#[test]
fn jimo_ai_draw_0_no_discard() {
    let (game, _, deck_before, hand_before) = setup(5, 0);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before);
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
}

/// 3 Aqours members, 1 hand card → draw 3 (now hand=4), discard 3 → hand=1 left.
/// Net hand = starting 1, net deck = -3.
#[test]
fn jimo_ai_draw_3_discard_3_limited_hand() {
    let (game, _, deck_before, hand_before) = setup(1, 3);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 3);
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
}
