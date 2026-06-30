/// Tests for 安養寺 姫芽 (PL!HS-bp1-009-R) — Debut look_and_select:
/// 登場 手札を1枚控え室に置いてもよい：
/// 自分のデッキの上からカードを5枚見る。その中から「みらくらぱーく！」の
/// カードを1枚公開して手札に加えてもよい。残りを控え室に置く。
use crate::helpers::*;

/// Edge: ド！ド！ド！ (live card, unit=みらくらぱーく！) among top 5 → selectable.
/// Must pay the hand-discard cost, then select the card to reveal.
#[test]
fn himeno_q82_dodo_live_card_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    let dodo = game.id("PL!HS-bp1-023-L");
    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    game.state.player1.main_deck.cards.insert(0, dodo);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    // Pay cost
    assert!(
        game.has_pending_choice(),
        "Should have optional cost choice"
    );
    game.select_indices(&[0]);

    // Look_and_select: select the matching card from looked_at
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    game.select_indices(&[0]);

    // Resolve any remaining sub-choices (reveal, move to hand, discard rest)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        game.state.player1.hand.cards.contains(&dodo),
        "Q82: ド！ド！ド！ (みらくらぱーく！) is selectable"
    );
}

/// Edge: アイデンティティ (live card, unit=みらくらぱーく！) selectable.
#[test]
fn himeno_q82_identity_live_card_selectable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    let identity = game.id("PL!HS-PR-012-PR");
    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    game.state.player1.main_deck.cards.insert(0, identity);
    for _ in 0..2 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    assert!(
        game.has_pending_choice(),
        "Should have optional cost choice"
    );
    game.select_indices(&[0]); // pay cost
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice"
    );
    game.select_indices(&[0]); // select identity
    while game.has_pending_choice() {
        game.select_indices(&[]);
    } // resolve remaining

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert!(
        game.state.player1.hand.cards.contains(&identity),
        "Q82: アイデンティティ (みらくらぱーく！) is selectable"
    );
}

/// Edge: No みらくらぱーく！ card among top 5 → nothing to reveal.
#[test]
fn himeno_edge_no_mirakura_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp1-009-R");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(himeno);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(4);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = -1;
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::LeftSide);

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    } // pay cost
      // Look_and_select shows 5 cards (group filter not applied at select level)
      // Consume the looked_at choice
    if game.has_pending_choice() {
        game.select_indices(&[]);
    } // skip looked_at selection

    assert!(!game.has_pending_choice(), "Ability should have ended");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "No cards in hand: himeno played, filler paid"
    );
}

// ====================================================================
//  安養寺 姫芽 (PL!HS-bp6-006) — Cost reduction & delayed cannot_active
// ====================================================================
// ab#0 (常時): Cost reduced by 2 per みらくらぱーく！ member on stage
// ab#1 (常時): Cannot be baton-touched by non-みらくらぱーく！ cards
// ab#2 (ライブ成功時): Wait this member + delayed cannot_active (1 turn)
//
// himeno bp6 base cost = 20
// ====================================================================

fn fill_basic_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

#[test]
fn himeno_bp6_cost_reduced_by_mirakura_on_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let mk = game.id("PL!HS-bp1-005-R"); // unit=みらくらぱーく！ member
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [mk, -1, -1];
    fill_basic_decks(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, -2, "1 Mirakura member on stage → cost -2");

    // Actually play the card: cost = 20 - 2 = 18
    game.give_energy(18);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::Center);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 18 energy consumed (20 base - 2 reduction)"
    );
}

#[test]
fn himeno_bp6_cost_no_reduction_without_mirakura() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [-1, -1, -1];
    fill_basic_decks(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, 0, "0 Mirakura members on stage → cost unchanged");

    // Actually play the card: cost = 20 - 0 = 20
    game.give_energy(20);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::Center);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 20 energy consumed (full base cost)"
    );
}

#[test]
fn himeno_bp6_cost_reduced_by_two_mirakura() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let mk1 = game.id("PL!HS-bp1-005-R"); // unit=みらくらぱーく！
    let mk2 = game.id("PL!HS-PR-005-PR"); // unit=みらくらぱーく！
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [mk1, mk2, -1];
    fill_basic_decks(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, -4, "2 Mirakura members → cost -4");

    // Actually play the card: cost = 20 - 4 = 16
    game.give_energy(16);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::RightSide);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 16 energy consumed (20 base - 4 reduction)"
    );
}

/// Q249: 3 みらくらぱーく！ on stage → baton touch one to debut 姫芽 → cost = 20 - 3×2 = 14
#[test]
fn himeno_bp6_q249_cost_reduced_by_three_mirakura() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let mk1 = game.id("PL!HS-bp1-005-R"); // unit=みらくらぱーく！
    let mk2 = game.id("PL!HS-PR-005-PR"); // unit=みらくらぱーく！
    let mk3 = game.id("PL!HS-bp1-005-R"); // another mirakura copy
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(himeno);
    game.state.player1.stage.stage = [mk1, mk2, mk3];
    fill_basic_decks(&mut game, filler);

    // Verify GameModifiers cost modifier
    game.state.recalculate_constant_cost_modifiers();
    let cost_mod = game.state.mods.get_cost_modifier(himeno);
    assert_eq!(cost_mod, -6, "3 Mirakura members on stage → cost -6");

    // Also verify via calculate_play_cost_reduction directly
    let reduction = rabuka_engine::ability::util::calculate_play_cost_reduction(
        &game.state.player1.stage,
        &game.state.player1.success_live_card_zone.cards,
        game.state.player1.hand.cards.len(),
        himeno,
        &game.db,
    );
    assert_eq!(
        reduction, 6,
        "calculate_play_cost_reduction = 6 for 3 mirakura"
    );

    // Play the card via baton touch (all 3 slots occupied).
    // Replacing Center (mk2, cost 10): cost = 20 - 6 - 10 = 4
    game.give_energy(4);
    game.play_to_stage(himeno, rabuka_engine::zones::MemberArea::Center);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "All 4 energy consumed (20 base - 6 reduction - 10 baton touch)"
    );
}

#[test]
fn himeno_bp6_delayed_cannot_active_blocks_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");
    let _filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = himeno;
    game.state.mods.add_orientation_modifier(himeno, "wait");

    game.state.mods.add_delayed_cannot_active(himeno, 1);

    // Check that is_delayed_cannot_active returns true
    assert!(game.state.mods.is_delayed_cannot_active(himeno));

    // Tick (simulate next Active phase processing)
    game.state.mods.tick_delayed_cannot_active();

    // After one tick, flag should be 0 → is_delayed_cannot_active returns false
    assert!(!game.state.mods.is_delayed_cannot_active(himeno));
}

#[test]
fn himeno_bp6_delayed_cannot_active_stack_resets_on_second_set() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let himeno = game.id("PL!HS-bp6-006-R＋");

    game.state.player1.stage.stage[0] = himeno;
    game.state.mods.add_orientation_modifier(himeno, "wait");

    // Set delayed flag twice — should keep max (not increase beyond 1)
    game.state.mods.add_delayed_cannot_active(himeno, 1);
    game.state.mods.add_delayed_cannot_active(himeno, 1);

    game.state.mods.tick_delayed_cannot_active(); // 1 → 0
    assert!(!game.state.mods.is_delayed_cannot_active(himeno));
}
