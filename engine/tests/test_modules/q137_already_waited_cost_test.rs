/// Tests for Q137: 既にウェイト状態のメンバーをコストで「ウェイトにする」ことはできません。
/// "No, you cannot. Putting into wait state means putting an active state member into wait state."
///
/// Covers two cards:
/// 1. PL!SP-bp4-002-R (唐 可可) — Debut optional self_cost wait + look_at_4
///    Cost: このメンバーをウェイトにしてもよい
///    Effect: look at top 4, pick Liella! live with 8+ hearts to hand, rest to discard.
/// 2. PL!-pb1-018-R (矢澤にこ) — Debut both-target: discard cost<=2 member → empty area in wait
///    Already-waited members on stage should not be targeted by effects that wait.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ── PL!SP-bp4-002-R (唐 可可) ──────────────────────────────────────

/// Q137-A: When Keke is active, paying the optional cost applies wait state
/// and the look_at_4 effect resolves.
#[test]
fn q137_keke_active_cost_pay_applies_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = game.id("PL!SP-bp4-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(keke);
    game.give_energy(10);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.play_to_stage(keke, MemberArea::LeftSide);

    // Keke is active → cost choice should be offered
    assert!(
        game.has_pending_choice(),
        "Should have optional cost choice when Keke is active"
    );
    game.select_option(1); // Pay

    let ori = game.state.mods.get_orientation_modifier(keke).cloned();
    assert_eq!(
        ori,
        Some("wait".to_string()),
        "Keke should be waited after paying cost"
    );

    // Effect resolves (look_at_4) — no valid Liella! live cards → skip selection
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Keke on stage, cost paid
    assert!(game.state.player1.stage.stage.contains(&keke));
}

/// Q137-B: Skipping the cost should not apply wait state and effect still resolves.
#[test]
fn q137_keke_skip_cost_no_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke = game.id("PL!SP-bp4-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(keke);
    game.give_energy(10);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.play_to_stage(keke, MemberArea::LeftSide);

    // Skip the cost
    assert!(game.has_pending_choice(), "Should have cost choice");
    game.select_option(0); // Skip

    // Resolve any remaining effect
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Keke should NOT be in wait state
    let ori = game.state.mods.get_orientation_modifier(keke).cloned();
    assert_eq!(ori, None, "Keke should NOT be waited after skipping cost");
}

/// Q137-C: The second copy of Keke on stage — when already-waited Keke is present,
/// the debut ability of the new (active) copy should still work normally.
/// The key check: only the activating member (the new copy) is considered for self_cost,
/// not the already-waited one.
#[test]
fn q137_keke_two_copies_only_active_one_costs() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let keke1 = game.id("PL!SP-bp4-002-R");
    let keke2 = game.new_id("PL!SP-bp4-002-R");
    let filler = game.id("PL!-sd1-010-SD");

    // Play first Keke and pay cost (becomes waited)
    game.state.player1.hand.cards.push(keke1);
    game.give_energy(10);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.play_to_stage(keke1, MemberArea::LeftSide);
    assert!(game.has_pending_choice());
    game.select_option(1); // Pay
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(keke1).cloned(),
        Some("wait".to_string()),
        "First Keke should be waited"
    );

    // Play second Keke — debut triggers, cost targets "this member" (keke2, active)
    game.state.player1.hand.cards.push(keke2);
    game.give_energy(10);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.play_to_stage(keke2, MemberArea::Center);

    // Cost should be offered for keke2 (active), NOT keke1 (already waited)
    assert!(
        game.has_pending_choice(),
        "Second Keke's cost should be offered (it is active)"
    );
    game.select_option(1); // Pay
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Both Keke on stage, both waited
    assert!(game.state.player1.stage.stage.contains(&keke1));
    assert!(game.state.player1.stage.stage.contains(&keke2));
    assert_eq!(
        game.state.mods.get_orientation_modifier(keke1).cloned(),
        Some("wait".to_string()),
        "First Keke still waited"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(keke2).cloned(),
        Some("wait".to_string()),
        "Second Keke should be waited"
    );
}

// ── PL!-pb1-018-R (矢澤にこ) ──────────────────────────────────────

/// Q137-D: Both players have eligible members in discard → both place in wait state.
/// Verifies the basic happy path of Nico's debut.
#[test]
fn q137_nico_both_have_eligible_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_p1 = game.id("PL!SP-sd1-019-SD");
    let cheap_p2 = game.id("PL!SP-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(cheap_p1);
    game.state.player2.waitroom.cards.push(cheap_p2);
    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;

    game.play_to_stage(nico, MemberArea::LeftSide);

    // P1 gets position choice
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P1 should get position choice"
    );
    game.select_option(1); // center

    // P2 gets position choice
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 should get position choice"
    );
    game.select_option(2); // right

    assert!(!game.has_pending_choice(), "No more pending choices");

    // P1: Nico at left, cheap_p1 at center in wait state
    assert_eq!(game.state.player1.stage.stage[0], nico);
    assert_eq!(game.state.player1.stage.stage[1], cheap_p1);
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p1),
        Some(&"wait".to_string()),
        "P1's placed member should be in wait state"
    );

    // P2: cheap_p2 at right in wait state
    assert_eq!(game.state.player2.stage.stage[2], cheap_p2);
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p2),
        Some(&"wait".to_string()),
        "P2's placed member should be in wait state"
    );
}

/// Q137-E: No eligible cards in either discard → both players skip gracefully.
#[test]
fn q137_nico_no_eligible_cards_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;

    game.play_to_stage(nico, MemberArea::LeftSide);

    // Q168: No suitable card in either discard → skip gracefully
    assert!(
        !game.has_pending_choice(),
        "No pending choice when both sides have no valid cards"
    );
    assert_eq!(game.state.player1.stage.stage[0], nico, "Nico at left");
    assert_eq!(game.state.player1.stage.stage[1], -1, "center empty");
    assert_eq!(game.state.player1.stage.stage[2], -1, "right empty");
}

/// Q137-F: P1 has no eligible card but P2 does → P2 gets choice, P1 skips.
#[test]
fn q137_nico_only_opponent_has_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nico = game.id("PL!-pb1-018-R");
    let cheap_p2 = game.id("PL!SP-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nico);
    game.state.player1.hand.cards.push(filler);
    game.state.player2.waitroom.cards.push(cheap_p2);
    game.give_energy(7);
    game.state.player1.stage.stage[0] = -1;

    game.play_to_stage(nico, MemberArea::LeftSide);

    // P2 gets position choice (P1 has no eligible cards, skips)
    assert_eq!(
        game.pending_choice_type(),
        Some("SelectPosition".to_string()),
        "P2 should get position choice"
    );
    game.select_option(0); // left

    assert!(!game.has_pending_choice());

    // P2's member placed at left in wait state
    assert_eq!(game.state.player2.stage.stage[0], cheap_p2);
    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap_p2),
        Some(&"wait".to_string()),
        "P2's placed member should be in wait state"
    );
}
