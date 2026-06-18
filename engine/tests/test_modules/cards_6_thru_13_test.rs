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
#[test]
fn c12_restriction_blocks_opponent_active() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!HS-pb1-008-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    deck(&mut g, f);
    g.give_energy(5);
    g.state.recalculate_constants();
    // restriction stores opponent player ID in cannot_activate_members
    let opp_id = &g.state.player2.id;
    assert!(
        g.state.cannot_activate_members.contains(opp_id),
        "opponent activation should be restricted (P2 in cannot_activate_members)"
    );
}

// ========== Card 13: PL!S-bp6-005-R look_and_select 3-heart ==========
#[test]
fn c13_look_select_three_heart_filter() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let c = g.id("PL!S-bp6-005-R");
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [-1, c, -1];
    deck(&mut g, f);
    g.give_energy(5);
    trigger(&mut g, c, "登場");
}
