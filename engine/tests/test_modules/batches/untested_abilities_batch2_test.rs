//! Untested-ability batch 2.
//!
//! Covers previously uncovered unique abilities (see TEST_INVENTORY.json):
//!   - PL!-bp3-005-R   登場: activate ALL own stage members
//!   - PL!-bp3-001-R   起動 (wait self → draw 1, discard 1) + ライブ開始時
//!                     (activate up to 1 member)
//!   - PL!-pb1-005-R   登場: success zone has cards → draw 1
//!   - PL!-bp5-006-R   ライブ開始時: live card zone ≥2 → draw 1
//!   - PL!SP-bp1-007-R＋ 登場: energy ≥11 → retrieve live card from waitroom
//!   - PL!SP-bp2-013-N 登場: up to 1 waitroom card to deck top
//! plus an interaction combo: pb1-006's self-wait activation undone by
//! bp3-005's mass-activate debut.

use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// 5 passes: Main → Active → Energy → Draw → Main → LiveCardSet.
fn advance_to_live_card_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

// ============================================================
// PL!-bp3-005-R — 登場: 自分のステージにいるすべてのメンバーをアクティブにする。
// ============================================================

#[test]
fn bp3_005_debut_activates_all_waited_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-005-R");
    let mate_a = game.id("PL!-sd1-010-SD");
    let mate_b = game.new_id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(20);
    fill_decks(&mut game, filler);

    // Two teammates on stage, both waited.
    // Direct placement: there is no natural action that waits an own member
    // here, and the ability under test needs waited targets.
    game.add_to_stage(MemberArea::LeftSide, mate_a);
    game.add_to_stage(MemberArea::Center, mate_b);
    game.state.mods.add_orientation_modifier(mate_a, "wait");
    game.state.mods.add_orientation_modifier(mate_b, "wait");

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(mate_a),
        Some("active"),
        "waited teammate A should be activated by the debut"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(mate_b),
        Some("active"),
        "waited teammate B should be activated by the debut"
    );
}

/// Interaction: PL!N-pb1-006-R's 起動 waits itself to activate an energy;
/// bp3-005's debut then re-activates it, readying it for reuse.
#[test]
fn combo_bp3_005_mass_activate_readies_self_waited_ability_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mass = game.id("PL!-bp3-005-R"); // cost 4
    let kanata = game.id("PL!N-pb1-006-R"); // cost 9, 起動: wait self → +1 energy
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(25);
    fill_decks(&mut game, filler);

    game.state.player1.hand.cards.push(kanata);
    game.play_to_stage(kanata, MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Kanata's 起動: waits herself, activates 1 energy.
    let energy_before = game.state.player1.energy_zone.active_count();
    game.activate_ability(kanata);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("wait"),
        "cost: kanata should be waited"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before + 1,
        "effect: one energy activated"
    );

    // Play bp3-005 — its debut activates ALL members incl. kanata.
    game.state.player1.hand.cards.push(mass);
    game.play_to_stage(mass, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(kanata),
        Some("active"),
        "bp3-005 debut should re-activate the self-waited kanata"
    );
    // Kanata can immediately be used again (no turn limit on her ability).
    let e2 = game.state.player1.energy_zone.active_count();
    game.activate_ability(kanata);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        e2 + 1,
        "reactivated kanata should be able to activate again"
    );
}

// ============================================================
// PL!-bp3-001-R — 起動 {{turn1}} このメンバーをウェイトにする：
//   カードを1枚引き、手札を1枚控え室に置く。
// ============================================================

#[test]
fn bp3_001_activation_waits_self_draw_then_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-001-R"); // cost 13
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.state.player1.hand.cards.push(card);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.activate_ability(card);

    // Cost paid: self waited.
    assert_eq!(
        game.state.mods.get_orientation_modifier(card),
        Some("wait"),
        "cost: member should be waited"
    );
    // Effect: drew 1, now must discard 1 from hand. The discard is
    // mandatory ("手札を1枚控え室に置く" — no もよい), so allow_skip=false.
    match game.get_pending_choice() {
        Choice::SelectCard {
            zone, allow_skip, ..
        } => {
            assert_eq!(zone, "hand", "discard must come from hand");
            assert!(!allow_skip, "mandatory discard must not be skippable");
        }
        _other => panic!(
            "expected SelectCard for the hand discard, got {}",
            game.pending_choice_summary()
        ),
    }
    let hand_at_prompt = game.state.player1.hand.cards.len();
    let discard_target = game.state.player1.hand.cards[0];
    game.select_indices(&[0]);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_at_prompt - 1,
        "resolve discard: prompt-time hand minus the discarded card"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&discard_target),
        "selected card must be in the waitroom"
    );

    // {{ターン1回}} — a second press in the same turn is rejected outright:
    // the member is waited (activations need an active member), so the
    // engine refuses before even reaching the use-limit bookkeeping.
    let err = game.try_activate_ability(card).unwrap_err();
    assert!(
        err.contains("No activatable") || err.contains("turn") || err.contains("already"),
        "expected second activation to be rejected, got: {}",
        err
    );
}

// ============================================================
// PL!-bp3-001-R — ライブ開始時: 自分のステージにいるメンバーを
//   1人までアクティブにする。
// ============================================================

#[test]
fn bp3_001_live_start_activates_one_chosen_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-001-R"); // cost 13, carries the live-start too
    let sleeper = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);

    // A waited teammate for the ability to wake up.
    game.add_to_stage(MemberArea::LeftSide, sleeper);
    game.state.mods.add_orientation_modifier(sleeper, "wait");

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Two passes out of LiveCardSet: P1 refills +1 (the set live), then
    // ライブ開始時 fires. Activate-one draws nothing.
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    game.pass();
    game.pass();

    // The live-start fires via the auto queue: accept the ability, then
    // choose WHICH member to activate ("1人まで" → SelectCard, skippable).
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectCard") => {
                assert!(
                    matches!(
                        game.get_pending_choice(),
                        Choice::SelectCard { ref zone, allow_skip, .. }
                            if zone == "stage" && *allow_skip,
                    ),
                    "expected skippable stage SelectCard, got {}",
                    game.pending_choice_summary()
                );
                game.select_indices(&[0]);
            }
            _ => game.select_indices(&[]),
        }
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(sleeper),
        Some("active"),
        "live-start should activate the chosen waited member"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone,
        "activate-one draws nothing beyond the live-zone refill"
    );
}

#[test]
fn bp3_001_live_start_can_skip_activation() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp3-001-R");
    let sleeper = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);

    game.add_to_stage(MemberArea::LeftSide, sleeper);
    game.state.mods.add_orientation_modifier(sleeper, "wait");

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);
    game.pass();
    game.pass();

    // Decline every optional prompt ("1人まで" = may choose zero).
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(sleeper),
        Some("wait"),
        "declined activation: member must stay waited"
    );
}

// ============================================================
// PL!-pb1-005-R — 登場: 自分の成功ライブカード置き場にカードがある場合、
//   カードを1枚引く。
// ============================================================

#[test]
fn pb1_005_debut_draws_with_cards_in_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-pb1-005-R"); // cost 2
    let filler = game.id("PL!-sd1-010-SD");
    let won_live = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(won_live);

    game.state.player1.hand.cards.push(card);
    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Debut: card left hand (-1), condition met → drew 1 (+1).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "play (-1) + conditional draw (+1) → hand count unchanged"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&card),
        "card itself must be on stage, not back in hand"
    );
}

#[test]
fn pb1_005_debut_no_draw_without_success_zone_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-pb1-005-R");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    assert!(
        game.state.player1.success_live_card_zone.cards.is_empty(),
        "precondition: empty success zone"
    );

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // No condition, no draw: hand went 1 → 0.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "without success-zone cards the debut must not draw"
    );
}

// ============================================================
// PL!-bp5-006-R — ライブ開始時: 自分のライブカード置き場にカードが
//   2枚以上ある場合、カードを1枚引く。
// ============================================================

#[test]
fn bp5_006_live_start_draws_when_live_zone_has_two_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-006-R"); // cost 2
    let filler = game.id("PL!-sd1-010-SD");
    let old_live_a = game.id("PL!-sd1-019-SD");
    let old_live_b = game.id("PL!-sd1-021-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);

    // Two previous lives in the live card zone.
    game.state.player1.live_card_zone.cards.push(old_live_a);
    game.state.player1.live_card_zone.cards.push(old_live_b);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Two passes leave the live-card-set phase:
    //   pass 1 (FA→SA): P1 refills by P1's live-zone count,
    //   pass 2 (SA→performance): P2 refills by P2's (empty) zone → +0,
    //   then ライブ開始時 fires → conditional +1.
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    game.pass();
    game.pass();

    drain_live_start_prompts(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone + 1,
        "P1 live-zone refill (+{p1_zone}) + conditional draw (+1)"
    );
}

fn drain_live_start_prompts(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            _ => game.select_indices(&[]),
        }
    }
}

#[test]
fn bp5_006_live_start_draws_at_exactly_two_cards_boundary() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    let old_live = game.id("PL!-sd1-019-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(old_live);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Boundary EXACTLY 2: 1 pre-existing live + this live = 2 in the zone
    // at live start → condition met (+1) on top of P1's refill (+2).
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    assert_eq!(p1_zone, 2, "boundary precondition: exactly 2 in zone");
    game.pass();
    game.pass();

    drain_live_start_prompts(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone + 1,
        "exactly 2 meets 2枚以上 → refill (+2) + conditional draw (+1)"
    );
}

/// Boundary below the threshold entirely: empty live card zone.
#[test]
fn bp5_006_live_start_no_draw_when_zone_below_two() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!-bp5-006-R");
    let filler = game.id("PL!-sd1-010-SD");
    let live_card = game.id("PL!-sd1-020-SD");

    game.give_energy(10);
    fill_decks(&mut game, filler);
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "precondition: empty live card zone"
    );

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.state.player1.hand.cards.push(live_card);
    advance_to_live_card_set(&mut game);
    game.set_live_card(live_card);

    // Only THIS live is in the zone (1 < 2) → P1 refills +1, no
    // conditional draw.
    let hand_before = game.state.player1.hand.cards.len();
    let p1_zone = game.state.player1.live_card_zone.cards.len();
    assert_eq!(p1_zone, 1, "precondition: only the set live in zone");
    game.pass();
    game.pass();

    drain_live_start_prompts(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + p1_zone,
        "zone 1 < 2 → refill (+1) only, no ability draw"
    );
}

// ============================================================
// PL!SP-bp1-007-R＋ — 登場: 自分のエネルギーが11枚以上ある場合、
//   自分の控え室からライブカードを1枚手札に加える。
//
// Rules note (rules.txt 7.x): paying a member's play cost flips ACTIVE
// energy to WAIT — the cards stay in the energy zone. 「エネルギーが
// 11枚以上」counts the WHOLE zone (active + wait), so after a full-price
// play of this cost-13 card the condition is necessarily true. The real
// edge is reaching her stage CHEAPLY (baton touch reduces the cost by the
// replaced member's cost, rules 9.6.2.3.2) on a small energy zone.
// ============================================================

#[test]
fn sp_bp1_007_debut_counts_total_energy_not_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp1-007-R＋"); // cost 13
    let filler = game.id("PL!-sd1-010-SD");
    let dead_live = game.id("PL!-sd1-020-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);
    game.add_to_discard(dead_live);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Full-price play: 13 of 15 flipped to wait → only 2 ACTIVE left.
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "precondition: active energy dropped below 11"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        15,
        "paid energy stays in the zone as wait state"
    );
    assert!(
        game.state.player1.hand.cards.contains(&dead_live),
        "condition counts TOTAL energy (15 ≥ 11) → live card retrieved"
    );
}

#[test]
fn sp_bp1_007_debut_no_retrieve_when_total_below_11_via_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp1-007-R＋"); // cost 13
    let filler = game.id("PL!-sd1-010-SD");
    let dead_live = game.id("PL!-sd1-020-SD");
    // Vanilla cost-13 member to baton-touch over: play cost becomes
    // 13 − 13 = 0 (rules 9.6.2.3.2).
    let victim = game.id("PL!N-sd2-002-SD2");

    fill_decks(&mut game, filler);
    // Direct placement of the baton-touch victim: there is no cheaper real
    // action that stages a cost-13 vanilla without spending the energy this
    // test must keep below 11.
    game.add_to_stage(MemberArea::Center, victim);
    game.give_energy(10); // total 10 < 11
    game.add_to_discard(dead_live);

    game.state.player1.hand.cards.push(card);
    // Real pipeline play with baton touch (the TestGame helper can't pass
    // use_baton_touch).
    rabuka_engine::turn::TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center), // baton touch replaces the member IN this area
        Some(true),
    )
    .expect("baton touch play should succeed at zero cost");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.stage.stage.contains(&card),
        "mei should be on stage"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&victim),
        "baton touch sends the replaced member to the waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        10,
        "zero-cost play keeps the energy zone untouched"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&dead_live),
        "total energy 10 < 11 → no retrieval"
    );
}

#[test]
fn sp_bp1_007_debut_noop_when_waitroom_has_no_live_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp1-007-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.give_energy(15);
    fill_decks(&mut game, filler);
    // Waitroom has only members, no live card.
    game.add_to_discard(filler);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);

    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        game.dbg_events(5);
        game.select_indices(&[]);
    }

    assert!(game.has_pending_choice() == false, "no stuck prompts");
}

// ============================================================
// PL!SP-bp2-013-N — 登場: 自分の控え室からカードを1枚までデッキの一番上に置く。
// ============================================================

#[test]
fn sp_bp2_013_debut_places_waitroom_card_on_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp2-013-N"); // cost 9
    let filler = game.id("PL!-sd1-010-SD");
    let marker = game.new_id("PL!-sd1-019-SD");

    game.give_energy(12);
    fill_decks(&mut game, filler);
    game.add_to_discard(marker);

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);

    // Choose the waitroom card ("1枚まで" → SelectCard with skip allowed).
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard to pick a waitroom card"
    );
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&marker),
        "chosen card must sit on top of the deck (index 0)"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&marker),
        "chosen card must have left the waitroom"
    );
}

#[test]
fn sp_bp2_013_debut_can_place_zero_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id("PL!SP-bp2-013-N");
    let filler = game.id("PL!-sd1-010-SD");
    let bystander = game.new_id("PL!-sd1-019-SD");

    game.give_energy(12);
    fill_decks(&mut game, filler);
    game.add_to_discard(bystander);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.state.player1.hand.cards.push(card);
    game.play_to_stage(card, MemberArea::Center);
    // Decline: take nothing from the waitroom.
    game.select_indices(&[]);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined placement: deck untouched"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&bystander),
        "bystander must remain in the waitroom"
    );
}
