use crate::helpers::*;

fn base_heart02(db: &rabuka_engine::card::CardDatabase, card_id: i16) -> u8 {
    db.get_card(card_id)
        .and_then(|c| c.base_heart.as_ref())
        .map(|bh| {
            bh.hearts
                .get(&rabuka_engine::card::HeartColor::Heart02)
                .copied()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

fn fill_both_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn drain_auto_choices(game: &mut TestGame) {
    while let Some(choice) = game.state.get_pending_choice() {
        match choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

// ======================================================================
// Fix 3: position+cost comparison with comparison_target="opponent"
// ======================================================================

/// Self cost(11) > Opponent cost(3) → condition passes → score+1 in snapshot
#[test]
fn opponent_cost_self_higher_scores_in_snapshot() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let self_center = game.id("PL!SP-pb1-001-R"); // cost=11
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage = [filler, self_center, filler];
    game.state.player2.stage.stage = [filler, game.id("PL!-sd1-010-SD"), filler];
    fill_both_decks(&mut game, filler);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    game.pass();

    assert!(
        game.state.performance_snapshots.iter().any(|snap| snap
            .breakdown
            .scores
            .iter()
            .any(|s| s.source.contains("スコア"))),
        "Self cost=11 > Opponent cost=3 → snapshot should contain +1 score"
    );
}

/// Self cost(3) < Opponent cost(11) → condition fails → no score line
#[test]
fn opponent_cost_self_lower_no_score_in_snapshot() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let filler = game.id("PL!-sd1-010-SD");
    let opp_center = game.id("PL!SP-pb1-001-R"); // cost=11

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage = [filler, filler, filler];
    game.state.player2.stage.stage = [filler, opp_center, filler];
    fill_both_decks(&mut game, filler);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    game.pass();

    assert!(
        !game.state.performance_snapshots.iter().any(|snap| snap
            .breakdown
            .scores
            .iter()
            .any(|s| s.source.contains("スコア"))),
        "Self cost=3 < Opponent cost=11 → snapshot should NOT contain score line"
    );
}

// ======================================================================
// Fix 4: resource_type=heart_NN with position filter
// Condition: left_side heart02 >= 3 → +2 blade to left_side member
// ======================================================================

/// Left_side heart02=4 >= 3 → condition passes → only left_side member gains +2 blade
/// Center and right side members must NOT receive blade even if they also have heart02=4.
#[test]
fn left_side_heart_meets_threshold_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let left_high = game.id("PL!SP-pb1-001-R"); // heart02=4
    let center_high = game.id("PL!SP-pb1-001-R"); // heart02=4
    let right_high = game.id("PL!SP-pb1-001-R"); // heart02=4
    assert_eq!(base_heart02(&game.db, left_high), 4);
    assert_eq!(base_heart02(&game.db, center_high), 4);
    assert_eq!(base_heart02(&game.db, right_high), 4);

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage = [left_high, center_high, right_high];
    fill_both_decks(&mut game, left_high);

    let left_blade_before = game.state.mods.get_blade_modifier(left_high);
    let center_blade_before = game.state.mods.get_blade_modifier(center_high);
    let right_blade_before = game.state.mods.get_blade_modifier(right_high);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    game.pass();

    assert_eq!(
        game.state.mods.get_blade_modifier(left_high) - left_blade_before,
        2,
        "Left_side heart02=4 >= 3 → should gain +2 blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(center_high),
        center_blade_before,
        "Center member must NOT gain blade — only left_side qualifies"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(right_high),
        right_blade_before,
        "Right member must NOT gain blade — only left_side qualifies"
    );
}

/// Left_side heart02=1, center heart02=4 (total=5, left_only=1).
/// Old code summed all positions → would PASS incorrectly.
/// Fixed code with position filter: left_only=1 < 3 → FAILS.
#[test]
fn left_side_heart_below_ignores_center_high() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let left_low = game.id("PL!SP-bp2-006-R\u{ff0b}"); // heart02=1
    let center_high = game.id("PL!SP-pb1-001-R"); // heart02=4
    let filler = game.id("PL!-sd1-010-SD");

    assert_eq!(base_heart02(&game.db, left_low), 1);
    assert_eq!(base_heart02(&game.db, center_high), 4);

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage = [left_low, center_high, filler];
    fill_both_decks(&mut game, filler);

    let blade_before = game.state.mods.get_blade_modifier(left_low);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    game.pass();

    assert_eq!(
        game.state.mods.get_blade_modifier(left_low),
        blade_before,
        "Left_side heart02=1 < 3, total stage=5 → position filter must reject"
    );
}

// ======================================================================
// Fix 2: gain_resource blade with position targets stage member, not live card
// ======================================================================

#[test]
fn blade_goes_to_stage_member_not_live_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let nonfiction = game.id("PL!SP-bp4-024-L");
    let left_high = game.id("PL!SP-pb1-001-R"); // heart02=4
    let filler = game.id("PL!-sd1-010-SD");

    assert_eq!(base_heart02(&game.db, left_high), 4);

    game.state.player1.hand.cards.push(nonfiction);
    game.state.player1.stage.stage = [left_high, filler, filler];
    fill_both_decks(&mut game, filler);

    let live_blade_before = game.state.mods.get_blade_modifier(nonfiction);
    let member_blade_before = game.state.mods.get_blade_modifier(left_high);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(nonfiction);
    game.pass();
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    game.pass();

    assert_eq!(
        game.state.mods.get_blade_modifier(nonfiction),
        live_blade_before,
        "Live card must NOT receive blade from position effect"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(left_high) - member_blade_before,
        2,
        "Left_side stage member must receive exactly +2 blade"
    );
}

// ======================================================================
// Fix 1: ability_applications routing — each player's apps stay in their
// own snapshot when both players have LiveStart abilities.
// ======================================================================

#[test]
fn both_players_get_own_live_start_effects() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let p1_card = game.id("PL!SP-bp4-024-L");
    let p2_card = game.new_id("PL!SP-bp4-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    let p_left = game.id("PL!SP-pb1-001-R"); // heart02=4, cost=11
    let p_center = game.id("PL!SP-pb1-001-R"); // cost=11
    let p2_left = game.new_id("PL!SP-pb1-001-R"); // heart02=4, cost=11
    let p2_center = game.new_id("PL!SP-pb1-001-R"); // cost=11

    game.state.player1.hand.cards.push(p1_card);
    game.state.player1.stage.stage = [p_left, p_center, filler];
    game.state.player2.hand.cards.push(p2_card);
    game.state.player2.stage.stage = [p2_left, p2_center, filler];
    fill_both_decks(&mut game, filler);

    let p1_blade_before = game.state.mods.get_blade_modifier(p_left);
    let p2_blade_before = game.state.mods.get_blade_modifier(p2_left);

    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(p1_card);
    game.pass();
    game.set_live_card(p2_card);
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    drain_auto_choices(&mut game);
    game.pass();
    drain_auto_choices(&mut game);

    // Both players got their own blade effect (not cross-consumed)
    assert_eq!(
        game.state.mods.get_blade_modifier(p_left) - p1_blade_before,
        2,
        "P1 left_side should gain +2 blade from its own LiveStart"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(p2_left) - p2_blade_before,
        2,
        "P2 left_side should gain +2 blade from its own LiveStart"
    );
}
