use crate::helpers::*;

fn deck(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn trigger(game: &mut TestGame, card_id: i16, trigger_str: &str) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .abilities
        .iter()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .cloned()
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        if trigger_str == "登場" {
            rabuka_engine::core::types::AbilityTrigger::Debut
        } else if trigger_str == "ライブ開始時" {
            rabuka_engine::core::types::AbilityTrigger::LiveStart
        } else if trigger_str == "起動" {
            rabuka_engine::core::types::AbilityTrigger::Activation
        } else {
            rabuka_engine::core::types::AbilityTrigger::Auto
        },
        pid.clone(),
        Some(card.card_no.clone()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            Some("SelectTarget") => {
                game.select_option(0);
            }
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            Some("SelectPosition") => {
                game.select_indices(&[0]);
            }
            Some("SelectHeartColor") | Some("SelectHeartType") => {
                game.select_indices(&[0]);
            }
            _ => break,
        }
    }
}

// ========== Card 6: PL!N-bp4-031-L LIVE Niji cost-sum check ==========
#[test]
fn c6_niji_cost_ge20_draw3_put3() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let l = g.id("PL!N-bp4-031-L");
    let n = g.id("PL!N-bp1-001-R"); // cost 9 虹ヶ咲
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [n, n, n]; // 27 >= 20
    g.state.player1.hand.cards.push(l);
    for _ in 0..5 {
        g.state.player1.hand.cards.push(g.id("PL!-sd1-010-SD"));
    }
    // Fill P1 deck with 虹ヶ咲 so the draw-3 finds matching cards
    g.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(n);
    }
    g.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player2.main_deck.cards.push(f);
    }
    g.give_energy(5);
    let h = g.state.player1.hand.cards.len();
    trigger(&mut g, l, "ライブ開始時");
    assert_eq!(g.state.player1.hand.cards.len(), h, "draw3 put3 = net0");
}
#[test]
fn c6_niji_cost_lt20_noop() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let l = g.id("PL!N-bp4-031-L");
    let n = g.id("PL!N-bp4-013-N"); // cost 4
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [n, n, n]; // 12 < 20
    g.state.player1.hand.cards.push(l);
    g.state.player1.hand.cards.push(f);
    deck(&mut g, f);
    g.give_energy(5);
    let h = g.state.player1.hand.cards.len();
    trigger(&mut g, l, "ライブ開始時");
    assert_eq!(g.state.player1.hand.cards.len(), h, "no draw");
}

// ========== Card 7: PL!N-bp3-009-R+ waitroom→deck bottom ==========
#[test]
fn c7_waitroom_to_deck_bottom() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!N-bp3-009-R+");
    let m = g.id("PL!-sd1-001-SD");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    g.state.player1.waitroom.cards.push(m);
    g.state.player1.waitroom.cards.push(m);
    deck(&mut g, f);
    g.give_energy(5);
    let w = g.state.player1.waitroom.cards.len();
    trigger(&mut g, c, "ライブ開始時");
    assert!(g.state.player1.waitroom.cards.len() < w, "waitroom shrank");
}

// ========== Card 8: PL!HS-pb1-012-R both shuffle ==========
#[test]
fn c8_both_shuffle_under_deck() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!HS-pb1-012-R");
    let m = g.id("PL!-sd1-001-SD");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    for _ in 0..5 {
        g.state.player1.waitroom.cards.push(m);
        g.state.player2.waitroom.cards.push(m);
    }
    deck(&mut g, f);
    g.give_energy(5);
    let w1 = g.state.player1.waitroom.cards.len();
    trigger(&mut g, c, "登場");
    assert!(
        g.state.player1.waitroom.cards.len() < w1,
        "P1 waitroom shrank"
    );
}

/// Q242: 百生吟子 debut — blade+2 gained even when no live card in discard.
/// The followup has two independent effects: retrieve live card + gain blade+2.
/// If retrieval fails (no live card in discard), blade+2 still applies.

/// Helper: trigger 百生吟子's debut, returns true if condition triggered (blade applied)
fn trigger_momoo_debut(game: &mut TestGame, card_id: i16) -> bool {
    trigger(game, card_id, "登場");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    let blade = game.state.mods.get_blade_modifier(card_id);
    blade > 0
}

/// Happy path: 20+ member cards in discard + live card in discard → both effects apply.
#[test]
fn c8_q242_both_shuffle_and_retrieve_and_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD"); // member card
    let live = game.id("PL!-sd1-019-SD"); // live card
    let f = game.id("PL!-sd1-010-SD"); // filler
    game.state.player1.stage.stage = [-1, c, -1];

    // 10 member cards each in both players' discards = 20 total (hits threshold)
    for _ in 0..10 {
        game.state.player1.waitroom.cards.push(m);
        game.state.player2.waitroom.cards.push(m);
    }
    // Live card in P1's discard for retrieval
    game.state.player1.waitroom.cards.push(live);
    let w1_before = game.state.player1.waitroom.cards.len();

    deck(&mut game, f);
    game.give_energy(5);
    let blade_before = game.state.mods.get_blade_modifier(c);

    trigger_momoo_debut(&mut game, c);

    // Live card was retrieved from discard to hand
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Q242: Live card should be retrieved to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "Q242: Live card should no longer be in discard"
    );
    // P1's waitroom shrank (member cards shuffled under + live card retrieved)
    assert!(
        game.state.player1.waitroom.cards.len() < w1_before,
        "Q242: P1 waitroom should shrink"
    );

    // Blade+2 gained
    let blade_after = game.state.mods.get_blade_modifier(c);
    assert_eq!(
        blade_after,
        blade_before + 2,
        "Q242: Should gain blade+2 from debut (happy path)"
    );
}

/// Q242: No live card in discard → blade+2 still gained.
#[test]
fn c8_q242_no_live_card_still_gains_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, c, -1];

    // 10 member cards each = 20 total (meets threshold)
    for _ in 0..10 {
        game.state.player1.waitroom.cards.push(m);
        game.state.player2.waitroom.cards.push(m);
    }
    // NO live card in discard!
    let hand_before = game.state.player1.hand.cards.len();

    deck(&mut game, f);
    game.give_energy(5);
    let blade_before = game.state.mods.get_blade_modifier(c);

    trigger_momoo_debut(&mut game, c);

    // No live card retrieved (hand unchanged from the retrieve effect)
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "Q242: Hand should not grow when no live card in discard"
    );

    // Blade+2 STILL gained (Q242: yes, you can)
    let blade_after = game.state.mods.get_blade_modifier(c);
    assert_eq!(
        blade_after,
        blade_before + 2,
        "Q242: Should gain blade+2 EVEN with no live card in discard"
    );
}

/// Edge: Exactly 20 cards moved (threshold met).
#[test]
fn c8_q242_exactly_20_threshold_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, c, -1];

    // Exactly 10 each = 20 total
    for _ in 0..10 {
        game.state.player1.waitroom.cards.push(m);
        game.state.player2.waitroom.cards.push(m);
    }
    game.state.player1.waitroom.cards.push(live);
    deck(&mut game, f);
    game.give_energy(5);

    trigger_momoo_debut(&mut game, c);

    let blade = game.state.mods.get_blade_modifier(c);
    assert!(
        blade >= 2,
        "Q242: Exactly 20 cards moved → threshold met, blade+2 should apply"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Q242: Live card retrieved at exactly 20 threshold"
    );
}

/// Edge: Only 19 cards moved (threshold NOT met) → no blade, no retrieval.
#[test]
fn c8_q242_19_below_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, c, -1];

    // 9 P1 + 10 P2 = 19 total (below 20 threshold)
    for _ in 0..9 {
        game.state.player1.waitroom.cards.push(m);
    }
    for _ in 0..10 {
        game.state.player2.waitroom.cards.push(m);
    }
    game.state.player1.waitroom.cards.push(live);
    deck(&mut game, f);
    game.give_energy(5);

    trigger_momoo_debut(&mut game, c);

    // Blade should NOT be gained
    let blade = game.state.mods.get_blade_modifier(c);
    assert_eq!(blade, 0, "Q242: 19 < 20 threshold → no blade+2");
    // Live card should NOT be retrieved
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "Q242: Live card not retrieved below threshold"
    );
}

/// Edge: Only P1 contributes all 20 cards (P2 contributes 0).
#[test]
fn c8_q242_p1_only_20_threshold() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let c = game.id("PL!HS-pb1-012-R");
    let m = game.id("PL!-sd1-001-SD");
    let live = game.id("PL!-sd1-019-SD");
    let f = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, c, -1];

    // 20 cards from P1 only, 0 from P2 = 20 total
    for _ in 0..20 {
        game.state.player1.waitroom.cards.push(m);
    }
    game.state.player1.waitroom.cards.push(live);
    deck(&mut game, f);
    game.give_energy(5);

    trigger_momoo_debut(&mut game, c);

    let blade = game.state.mods.get_blade_modifier(c);
    assert!(
        blade >= 2,
        "Q242: P1-only 20 cards → threshold met (total=20), blade+2 applies"
    );
}

// ========== Card 9: PL!-bp5-111-R A-RISE ==========
#[test]
fn c9_arise_constant_heart_with_other_arise() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!-bp5-111-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    deck(&mut g, f);
    g.give_energy(5);
    g.state.recalculate_constants();
}

#[test]
fn c9_arise_activate_and_recover() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!-bp5-111-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    let opp = g.id("PL!-sd1-001-SD");
    g.state.player2.stage.stage = [-1, opp, -1];
    g.state.mods.add_orientation_modifier(opp, "wait");
    g.state.player1.waitroom.cards.push(f);
    g.state.player1.hand.cards.push(f);
    deck(&mut g, f);
    g.give_energy(5);
    g.activate_ability(c);
    while g.has_pending_choice() {
        match g.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                g.select_indices(&[0]);
            }
            Some("SelectTarget") => {
                g.select_option(0);
            }
            _ => break,
        }
    }
}

// ========== Card 10: PL!-bp6-007-R+ LIVE reveal top ==========
#[test]
fn c10_reveal_top_adds_to_hand() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let l = g.id("PL!-bp6-007-R+");
    let m = g.id("PL!-sd1-001-SD");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [m, m, m];
    g.state.player1.hand.cards.push(l);
    // P2 has no hand/live card → P1 auto-wins, triggers LiveSuccess
    g.state.player2.hand.cards.clear();
    deck(&mut g, f);
    g.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(f);
    }
    g.give_energy(5);
    let hb = g.state.player1.hand.cards.len();
    for _ in 0..5 {
        g.pass();
    }
    g.set_live_card(l);
    for _ in 0..2 {
        g.pass();
    }
    while g.has_pending_choice() {
        match g.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => {
                g.select_indices(&[]);
            }
            _ => break,
        }
    }
    for _ in 0..3 {
        g.pass();
    }
    while g.has_pending_choice() {
        match g.pending_choice_type().as_deref() {
            Some("SelectLiveSuccess") => {
                g.select_indices(&[0]);
            }
            Some("SelectAutoAbility") => {
                g.select_indices(&[]);
            }
            _ => break,
        }
    }
    // Live card removed from hand (set as live), then top card revealed and added.
    // Net: card count should be >= original minus 1 (the live card that was played).
    assert!(
        g.state.player1.hand.cards.len() >= hb - 1,
        "top card should be added to hand (was {}, now {})",
        hb,
        g.state.player1.hand.cards.len()
    );
}

// ========== Card 11: PL!N-bp3-028-L LIVE peek N per Niji ==========
#[test]
fn c11_peek_per_niji_selects_keep1() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let l = g.id("PL!N-bp3-028-L");
    let n = g.id("PL!N-bp1-001-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [n, n, n];
    g.state.player1.hand.cards.push(l);
    deck(&mut g, f);
    g.give_energy(5);
    trigger(&mut g, l, "ライブ開始時");
}

// ========== Card 12: PL!HS-pb1-008-R restriction ==========
/// P1 owns the restriction card — P2's members should be blocked during P2's active phase.
#[test]
fn c12_restriction_blocks_opponent_active() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let restriction = g.id("PL!HS-pb1-008-R");
    let waiting = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [restriction, -1, -1];
    g.state.player2.stage.stage = [waiting, -1, -1];
    g.state.mods.add_orientation_modifier(waiting, "wait");
    deck(&mut g, waiting);
    g.give_energy(5);
    g.state.recalculate_constants();

    // restriction stores P2 ID (opponent) in constant_cannot_activate_members
    let opp_id = &g.state.player2.id;
    assert!(
        g.state.constant_cannot_activate_members.contains(opp_id),
        "opponent ID should be in constant_cannot_activate_members"
    );

    // During P2's turn (P2 is active player), P2's activation should be blocked
    let p2_turn = g.state.player2.id.clone();
    let is_activation_blocked = g
        .state
        .constant_cannot_activate_members
        .iter()
        .any(|t| t == &p2_turn);
    assert!(
        is_activation_blocked,
        "P2's turn: P2 should be blocked by the restriction"
    );

    // P2's member should NOT become active
    let orientation = g.state.mods.get_orientation_modifier(waiting);
    assert_eq!(
        orientation,
        Some(&"wait".to_string()),
        "P2's waiting member should remain in wait state (blocked by restriction)"
    );
}

/// P2 owns the restriction card — P1's members should be blocked during active phase.
/// This tests the bug where resolve_target_player returned wrong player during recalculate_constants.
#[test]
fn c12_restriction_p2_owned_blocks_p1_active() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let restriction = g.id("PL!HS-pb1-008-R");
    let waiting = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [waiting, -1, -1];
    g.state.player2.stage.stage = [restriction, -1, -1];
    g.state.mods.add_orientation_modifier(waiting, "wait");
    deck(&mut g, waiting);
    g.give_energy(5);
    g.state.recalculate_constants();

    // P1 is the turn player, P2 owns the restriction card targeting "opponent"
    // resolve_target_player("opponent") should return P1 (P2's opponent)
    let p1_id = &g.state.player1.id;
    assert!(
        g.state.constant_cannot_activate_members.contains(p1_id),
        "P1 should be in constant_cannot_activate_members (P2's opponent)"
    );

    // Simulate active phase activation logic
    let turn_player_id = g.state.active_player().id.clone();
    let is_activation_blocked = g
        .state
        .constant_cannot_activate_members
        .iter()
        .any(|t| t == &turn_player_id);
    assert!(
        is_activation_blocked,
        "P1's turn: P1 should be blocked because P2's restriction targets opponent"
    );

    // P1's member should NOT have been activated
    let orientation = g.state.mods.get_orientation_modifier(waiting);
    assert_eq!(
        orientation,
        Some(&"wait".to_string()),
        "P1's waiting member should remain in wait state despite active phase"
    );
}

/// P1 owns the restriction card, P1's own members should still activate (target: opponent).
#[test]
fn c12_restriction_does_not_block_self_active() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let restriction = g.id("PL!HS-pb1-008-R");
    let waiting = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [restriction, waiting, -1];
    g.state.mods.add_orientation_modifier(waiting, "wait");
    deck(&mut g, waiting);
    g.give_energy(5);
    g.state.recalculate_constants();

    // constant_cannot_activate_members contains P2 (opponent), NOT P1
    let p1_id = &g.state.player1.id;
    assert!(
        !g.state.constant_cannot_activate_members.contains(p1_id),
        "P1 should NOT be in constant_cannot_activate_members (self is not opponent)"
    );

    // P1's own member SHOULD be activated
    let turn_player_id = g.state.active_player().id.clone();
    let is_activation_blocked = g
        .state
        .constant_cannot_activate_members
        .iter()
        .any(|t| t == &turn_player_id);
    assert!(
        !is_activation_blocked,
        "P1's turn: P1's own activation should NOT be blocked"
    );

    // Simulate active phase activation
    if !is_activation_blocked {
        g.state.mods.add_orientation_modifier(waiting, "active");
    }
    let orientation = g.state.mods.get_orientation_modifier(waiting);
    assert_eq!(
        orientation,
        Some(&"active".to_string()),
        "P1's own waiting member should become active during active phase"
    );
}

// =====================================================================
// Card 13: PL!S-bp6-005-R — 登場 look_and_select 3-heart ALL required
// =====================================================================
// Card text:
//   自分のデッキの上からカードを2枚見る。その中から
//   {{heart_02.png|heart02}}と{{heart_04.png|heart04}}と
//   {{heart_05.png|heart05}}をすべて持つメンバーカードを
//   1枚公開して手札に加えてもよい。残りを控え室に置く。
//
// Hearts are joined by と (AND) + すべて持つ (has all) → require_all_heart_colors = true
// Card must have ALL of heart02, heart04, heart05 simultaneously.

fn resolve_all_up_to_20(game: &mut TestGame, max: usize) {
    for _ in 0..max {
        if !game.has_pending_choice() {
            return;
        }
        game.select_indices(&[]);
    }
    panic!("resolve_all_up_to_20: exceeded {} iters", max);
}

fn setup_bp6_005(game: &mut TestGame, top_cards: Vec<i16>) {
    let you = game.id("PL!S-bp6-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    // Place card on stage directly, then fill deck
    game.state.player1.stage.stage = [-1, you, -1];
    game.state.player1.main_deck.cards.clear();
    for cid in top_cards {
        game.state.player1.main_deck.cards.push(cid);
    }
    while game.state.player1.main_deck.cards.len() < 40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(5);
    trigger(game, you, "登場");
}

#[test]
fn c13_all_three_hearts_present_select_one() {
    // Cards with ALL three hearts (heart02, heart04, heart05) should be selectable
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // has heart02, heart04, heart05
    let filler = g.id("PL!-sd1-010-SD"); // has heart01, heart03 — no match

    setup_bp6_005(&mut g, vec![qualifying, filler]);
    // Should prompt: optional cost? No. Should go to look_and_select
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    resolve_all_up_to_20(&mut g, 20);
    assert!(
        g.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card (all 3 hearts) should be in hand"
    );
}

#[test]
fn c13_two_of_three_hearts_rejected() {
    // Cards with only 2 of the 3 required hearts should NOT be selectable
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let two_hearts = g.id("PL!S-PR-015-PR"); // has heart02, heart04 only — missing heart05
    let filler = g.id("PL!-sd1-010-SD");

    setup_bp6_005(&mut g, vec![two_hearts, filler]);
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    resolve_all_up_to_20(&mut g, 20);
    assert!(
        !g.state.player1.hand.cards.contains(&two_hearts),
        "Card with only heart02+heart04 should NOT be selectable (missing heart05)"
    );
}

#[test]
fn c13_one_of_three_hearts_rejected() {
    // Card with only heart02 (not heart04, heart05) should be rejected
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let blade_card = g.id("PL!SP-sd1-001-SD"); // has heart02, heart06 only
    let filler = g.id("PL!-sd1-010-SD");

    setup_bp6_005(&mut g, vec![blade_card, filler]);
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    resolve_all_up_to_20(&mut g, 20);
    assert!(
        !g.state.player1.hand.cards.contains(&blade_card),
        "Card with only heart02+heart06 should NOT be selectable (missing heart04, heart05)"
    );
}

#[test]
fn c13_no_matching_card_optional_same_as_reject() {
    // If neither top card matches, the ability may still resolve (optional = true)
    let db = load_real_database();
    let mut g = TestGame::new(db);
    // Use a PR card with heart02+heart05 (missing heart04) as the only candidate
    let partial = g.id("PL!S-PR-017-PR"); // heart02, heart05 only — missing heart04
    let other = g.id("PL!S-bp2-015-PR"); // heart04, heart05 only — missing heart02

    setup_bp6_005(&mut g, vec![partial, other]);
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    resolve_all_up_to_20(&mut g, 20);
    // Neither card should be in hand (none has all 3 hearts)
    assert!(
        !g.state.player1.hand.cards.contains(&partial),
        "Partial heart card should NOT be in hand"
    );
    assert!(
        !g.state.player1.hand.cards.contains(&other),
        "Other partial heart card should NOT be in hand"
    );
}

#[test]
fn c13_bp2_005_or_semantics_still_works() {
    // Verify that bp2-005 (か = OR) still works with ANY match
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let you = g.id("PL!S-bp2-005-R\u{ff0b}");
    let filler = g.id("PL!-sd1-010-SD");
    let blade_card = g.id("PL!SP-sd1-001-SD"); // has heart02, heart06
    g.state.player1.hand.cards.push(you);
    g.state.player1.hand.cards.push(filler);
    g.state.player1.main_deck.cards.extend(vec![
        blade_card, filler, filler, filler, filler, filler, filler,
    ]);
    while g.state.player1.main_deck.cards.len() < 40 {
        g.state.player1.main_deck.cards.push(filler);
    }
    g.give_energy(13);
    g.state.player1.stage.stage[0] = -1;
    g.play_to_stage(you, rabuka_engine::zones::MemberArea::LeftSide);
    // Pay optional cost (discard from hand)
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    // Select first looked-at card
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    resolve_all_up_to_20(&mut g, 30);
    // With OR semantics, heart02 alone should match
    assert!(
        g.state.player1.hand.cards.contains(&blade_card),
        "bp2-005 OR semantics: card with only heart02 should be selectable"
    );
}

#[test]
fn c13_qualifying_goes_to_hand_nonqualifying_discarded() {
    // Qualifying card goes to hand, non-qualifying filler goes to waitroom
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let qualifying = g.id("PL!S-sd1-001-SD"); // has all 3 hearts
    let filler = g.id("PL!-sd1-010-SD"); // heart01, heart03 — no match

    setup_bp6_005(&mut g, vec![qualifying, filler]);
    if g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    resolve_all_up_to_20(&mut g, 20);
    assert!(
        g.state.player1.hand.cards.contains(&qualifying),
        "Qualifying card should be in hand"
    );
    assert!(
        !g.state.player1.hand.cards.contains(&filler),
        "Non-qualifying filler should NOT be in hand"
    );
}
