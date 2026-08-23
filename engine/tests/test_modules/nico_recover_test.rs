/// Tests for 矢澤にこ (PL!-bp5-009-R) — Activation ability:
/// 起動（ターン1回）手札2枚控え室に置く：
/// 控え室からブレードハートにheart06が3以上含むライブカード1枚を手札に加える
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

fn setup_nico(game: &mut TestGame) -> i16 {
    let nico = game.id("PL!-bp5-009-R");
    game.state.player1.stage.stage[1] = nico;
    game.give_energy(15);
    nico
}

fn activate_nico(game: &mut TestGame, nico: i16) {
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(nico),
        None,
        None,
        None,
    )
    .expect("activate nico");
}

/// Answer the mandatory 2-card discard cost by picking the named cards
/// (raw hand indices). Panics if the cost prompt is missing or mis-shaped.
fn pay_discard_cost(game: &mut TestGame, cards: &[i16]) {
    assert!(
        game.has_pending_choice(),
        "discard cost must prompt immediately (got: {})",
        game.pending_choice_summary()
    );
    match game.get_pending_choice() {
        Choice::SelectCard { zone, count, allow_skip, .. } => {
            assert_eq!(zone, "hand", "cost source is the hand");
            assert_eq!(*count, 2, "cost is exactly 2 cards");
            assert!(!*allow_skip, "cost is mandatory");
        }
        other => panic!("expected SelectCard cost, got {:?}", other),
    }
    let mut idxs = Vec::new();
    for c in cards {
        let pos = game
            .state
            .player1
            .hand
            .cards
            .iter()
            .position(|&x| x == *c)
            .unwrap_or_else(|| panic!("cost card still in hand"));
        idxs.push(pos);
    }
    idxs.sort();
    idxs.dedup();
    game.select_indices(&idxs);
}

fn drain_to_completion(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { .. } => game.select_indices(&[0]),
            _ => break,
        }
    }
}

/// Eligible live (blade_heart with heart06 >= 3) pre-seeded in the waitroom,
/// cost paid with 2 fillers → exactly that card is recovered.
#[test]
fn nico_recover_with_heart06_ge3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = setup_nico(&mut game);

    let filler_a = game.id("PL!-sd1-010-SD");
    let filler_b = game.new_id("PL!-sd1-010-SD");
    let filler_c = game.new_id("PL!-sd1-010-SD");
    let filler_d = game.id("PL!-sd1-010-SD"); // stays in hand
    for f in [&filler_a, &filler_b, &filler_c, &filler_d] {
        game.add_to_hand(*f);
    }
    let eligible = game.id("PL!N-bp1-028-L");
    game.add_to_discard(eligible);

    activate_nico(&mut game, nico);
    pay_discard_cost(&mut game, &[filler_a, filler_b]);
    // Exactly ONE matching live exists → the engine auto-selects it
    // (no recovery prompt). drain resolves any trailing prompts.
    drain_to_completion(&mut game);

    assert!(
        game.state.player1.hand.cards.contains(&eligible),
        "eligible live recovered to hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&filler_a)
            && game.state.player1.waitroom.cards.contains(&filler_b),
        "BOTH cost cards reached the waitroom"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        3,
        "-2 cost +1 recovered"
    );
}

/// An ineligible live sitting in the waitroom must NOT be recovered even
/// though it is present — the heart06>=3 blade-heart filter rejects it.
#[test]
fn nico_heart06_too_low_not_recovered() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = setup_nico(&mut game);

    let fillers: Vec<i16> = (0..4).map(|_| game.new_id("PL!-sd1-010-SD")).collect();
    for f in &fillers {
        game.add_to_hand(*f);
    }
    let ineligible_live = game.id("PL!-sd1-019-SD"); // live, no heart06 blade-heart
    game.add_to_discard(ineligible_live);

    activate_nico(&mut game, nico);
    pay_discard_cost(&mut game, &[fillers[0], fillers[1]]);
    // No matching card → no recovery prompt appears at all.
    drain_to_completion(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "only the 2 cost cards left the hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&ineligible_live),
        "ineligible live STAYS in the waitroom"
    );
    assert!(
        !game.has_pending_choice(),
        "no recovery prompt when nothing matches"
    );
}

/// Empty-matching waitroom → pure cost payment, nothing recovered.
#[test]
fn nico_no_eligible_card_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = setup_nico(&mut game);

    let fillers: Vec<i16> = (0..4).map(|_| game.new_id("PL!-sd1-010-SD")).collect();
    for f in &fillers {
        game.add_to_hand(*f);
    }

    activate_nico(&mut game, nico);
    pay_discard_cost(&mut game, &[fillers[0], fillers[1]]);
    drain_to_completion(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 2);
    assert!(
        !game.has_pending_choice(),
        "no recovery prompt when the waitroom has no live cards"
    );
}

/// Multiple eligible lives in the waitroom → the player chooses ONE;
/// the other stays behind.
#[test]
fn nico_one_card_recovered_even_if_multiple_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = setup_nico(&mut game);

    let fillers: Vec<i16> = (0..4).map(|_| game.new_id("PL!-sd1-010-SD")).collect();
    for f in &fillers {
        game.add_to_hand(*f);
    }
    let eligible_a = game.id("PL!N-bp1-028-L");
    let eligible_b = game.new_id("PL!N-bp1-028-L");
    game.add_to_discard(eligible_a);
    game.add_to_discard(eligible_b);

    activate_nico(&mut game, nico);
    pay_discard_cost(&mut game, &[fillers[0], fillers[1]]);
    // Recovery choice offers BOTH eligible cards; take eligible_a.
    game.select_waitroom_card_filtered(eligible_a);
    drain_to_completion(&mut game);

    assert!(
        game.state.player1.hand.cards.contains(&eligible_a),
        "chosen eligible recovered"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&eligible_b),
        "the OTHER eligible stays in the waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&eligible_a),
        "chosen eligible left the waitroom"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 3);
}
