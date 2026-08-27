/// Tests for LL-bpX-001-R＋ multi-name joint cards
///
/// These cards (bp1,bp2,bp3,bp4,bp6) each have a ライブ開始時 ability with
/// subtleties the engine previously mishandled:
///
/// - bp1 (上原歩夢＆澁谷かのん＆日野下花帆):
///     Cost=discard exactly 3 matching name cards (any combo including self).
///     Effect=gain「常時スコア+3」ability until live end.
///
/// - bp2 (渡辺曜＆鬼塚夏美＆大沢瑠璃乃):
///     Cost=discard ANY NUMBER of matching (including self).
///     Effect=gain 1 blade per discarded card until live end.
///
/// - bp3 (園田海未＆津島善子＆天王寺璃奈):
///     ライブ開始時: pay 6E optional → gain 3 blade until live end.
///     (Energy-only cost, simpler, but still exercises the trigger path.)
///
/// - bp4 (絢瀬絵里＆朝香果林＆葉月恋):
///     triggers="ライブ開始時, 登場" (DUAL trigger) — previously the engine
///     used == instead of .contains() so the live_start NEVER fired.
///     Effect=look top 5, optionally take one of the three named members,
///     then wait opponent members ≤ revealed card cost with ≤3 blade.
///
/// - bp6 (南ことり＆黒澤ダイヤ＆徒町小鈴):
///     Cost=discard ANY NUMBER of matching (including self).
///     Effect=for each distinct heart COLOR among discarded cards, gain 1 of that color.
///
use crate::helpers::*;
use rabuka_engine::{card::HeartColor, zones::MemberArea};

// ─────────────────────────────────────────────────────────────
// Shared live-start advance helpers
// ─────────────────────────────────────────────────────────────
fn advance_to_live_start(game: &mut TestGame) {
    // From initial phase: pass 5 times to reach LiveCardSetP1
    game.pass(); // → ActivePhase
    game.pass(); // → EnergyPhase
    game.pass(); // → DrawPhase
    game.pass(); // → MainPhase
    game.pass(); // → LiveCardSetP1
                 // Then P1 sets live card via set_live_card() externally
}

fn finish_live_setup(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1 → LiveCardSetP2
    game.pass(); // LiveCardSetP2 → LiveStart (triggers fire here)
}

// ─────────────────────────────────────────────────────────────
// bp1 — 上原歩夢＆澁谷かのん＆日野下花帆
// ライブ開始時: discard exactly 3 cards whose name is any of the three → gain score+3
// ─────────────────────────────────────────────────────────────

/// The multi-name card itself counts as all three names for the cost filter.
/// One copy in hand + 2 fillers whose names match should satisfy count=3.
/// Selecting all 3 should pay cost and grant score+3.
#[test]
fn test_bp1_live_start_pay_cost_gains_score_three() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp1-001-R\u{ff0b}"); // 上原歩夢&澁谷かのん&日野下花帆
    let ayumu = game.id("PL!N-sd1-001-SD"); // 上原歩夢
    let kanon = game.id("PL!SP-sd1-001-SD"); // 澁谷かのん
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    // Stage: joint in center
    game.state.player1.stage.stage[1] = joint;
    // Hand: 3 matching names (joint self + ayumu + kanon) to pay cost
    game.state.player1.hand.cards.push(joint);
    game.state.player1.hand.cards.push(ayumu);
    game.state.player1.hand.cards.push(kanon);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp1 live_start should prompt to select 3 cards for optional cost"
    );

    // Select all 3 matching cards (indices in the choice list)
    while game.has_pending_choice() {
        game.try_select_indices(&[0, 1, 2]).unwrap();
    }

    assert!(
        !game.has_pending_choice(),
        "bp1 should resolve after cost payment"
    );

    // Effect: score+3 modifier applied to joint
    game.state.recalculate_constants();
        let score_mod = game.state.mods.p1_constant_total_score_bonus;
    assert_eq!(
        score_mod, 3,
        "bp1: paying cost should grant score+3 (got {})",
        score_mod
    );
}

/// If there are no matching name cards in hand, the optional cost is skipped
/// and no effect fires.
#[test]
fn test_bp1_live_start_no_matching_hand_cards_skips() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp1-001-R\u{ff0b}");
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    // Hand: only unrelated fillers — no matching names
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    // Add live card to hand so it can be set
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Optional cost with no eligible cards → engine skips automatically
    assert!(
        !game.has_pending_choice(),
        "bp1 should auto-skip when there are no matching-name cards in hand"
    );
}

// ─────────────────────────────────────────────────────────────
// bp2 — 渡辺曜＆鬼塚夏美＆大沢瑠璃乃
// ライブ開始時: discard ANY NUMBER of matching → 1 blade per discarded
// ─────────────────────────────────────────────────────────────

/// Discarding 2 matching cards should grant 2 blades.
#[test]
fn test_bp2_live_start_discard_any_number_gains_blade_per_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp2-001-R\u{ff0b}"); // 渡辺曜&鬼塚夏美&大沢瑠璃乃
    let you = game.id("PL!S-sd1-005-SD"); // 渡辺 曜
    let natsumi = game.id("PL!SP-sd1-009-SD"); // 鬼塚夏美
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    // Put 2 matching-name cards in hand (not the joint card itself)
    game.add_to_hand(you);
    game.add_to_hand(natsumi);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // Should prompt to discard "any number" (0-2)
    assert!(
        game.has_pending_choice(),
        "bp2 live_start should prompt for any-number discard"
    );

    // Select both matching cards (indices 0 and 1)
    game.try_select_indices(&[0, 1]).unwrap();
    game.select_indices(&[]); // skip re-prompt, finalize

    assert!(
        !game.has_pending_choice(),
        "bp2 should resolve after card selection"
    );

    // 2 cards discarded → 2 blades gained by joint
    let blades = game.state.mods.get_blade_modifier(joint);
    assert_eq!(
        blades, 2,
        "bp2: discarding 2 cards should grant 2 blades (got {})",
        blades
    );
}

/// Skipping the optional cost (0 cards) grants 0 blades.
#[test]
fn test_bp2_live_start_skip_cost_gains_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp2-001-R\u{ff0b}");
    let you = game.id("PL!S-sd1-005-SD"); // 渡辺 曜
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(you);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp2 should prompt for card selection"
    );

    // Skip — select no cards
    game.select_indices(&[]);

    assert!(!game.has_pending_choice(), "Should resolve after skip");

    let blades = game.state.mods.get_blade_modifier(joint);
    assert_eq!(blades, 0, "Skipping bp2 cost should give 0 blades");
}

// ─────────────────────────────────────────────────────────────
// bp3 — 園田海未＆津島善子＆天王寺璃奈
// ライブ開始時: pay 6E optional → gain 3 blade until live end
// ─────────────────────────────────────────────────────────────

/// Paying 6 energy grants 3 blades.
#[test]
fn test_bp3_live_start_pay_6e_gains_3_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp3-001-R\u{ff0b}"); // 園田海未&津島善子&天王寺璃奈
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10); // more than enough

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp3 should prompt to pay 6E (or skip)"
    );

    // Pay the optional cost
    game.select_option(1); // index 1 = "Pay"

    assert!(
        !game.has_pending_choice(),
        "bp3 should resolve after paying energy"
    );

    let blades = game.state.mods.get_blade_modifier(joint);
    assert_eq!(
        blades, 3,
        "bp3: paying 6E should grant 3 blades (got {})",
        blades
    );

    // Energy should have decreased by 6
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4, // 10 - 6
        "6 energy should have been paid"
    );
}

/// Skipping the 6E cost gives 0 blades.
#[test]
fn test_bp3_live_start_skip_gains_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp3-001-R\u{ff0b}");
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp3 should prompt for optional cost"
    );

    game.select_option(0); // Skip

    let blades = game.state.mods.get_blade_modifier(joint);
    assert_eq!(blades, 0, "Skipping bp3 cost gives 0 blades");
}

// ─────────────────────────────────────────────────────────────
// bp4 — 絢瀬絵里＆朝香果林＆葉月恋
// triggers="ライブ開始時, 登場" — dual trigger, previously broken
// Effect: look top 5, optionally take 1 named member to hand, discard rest,
//         then wait all opponent members ≤ revealed cost with ≤3 original blade.
// ─────────────────────────────────────────────────────────────

/// The dual-trigger ability MUST fire on ライブ開始時.
/// Previously the engine did `triggers == "ライブ開始時"` (exact match),
/// which missed `"ライブ開始時, 登場"`. This test verifies the fix.
#[test]
fn test_bp4_live_start_dual_trigger_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp4-001-R\u{ff0b}"); // 絢瀬絵里&朝香果林&葉月恋
    let live = game.id("PL!-sd1-010-SD");
    let ayumu = game.id("PL!-pb1-011-R"); // 絢瀬絵里 — matches select filter
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(live);
    // Only ~1 draw happens before live_start triggers. Place ayumu in
    // position 3 so after 1 draw it's still in the top-5 looked-at cards.
    game.state.player1.main_deck.cards.clear();
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.push(ayumu); // positioned at index 3
    for _ in 0..7 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    // The dual-trigger ability should have fired and produced a pending choice
    // (look-at-5 then select from them)
    assert!(
        game.has_pending_choice(),
        "bp4 live_start ability MUST fire (dual trigger fix). Previously broken."
    );
}

/// bp4 debut trigger still works independently.
#[test]
fn test_bp4_debut_trigger_fires_independently() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp4-001-R\u{ff0b}");
    let ayumu = game.id("PL!-pb1-011-R"); // 絢瀬絵里 — matches select filter
    let filler = game.new_id("PL!-sd1-010-SD");

    game.add_to_hand(joint);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(ayumu);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(20);

    // Play joint to stage — this fires 登場 trigger
    game.play_to_stage(joint, MemberArea::Center);

    // Should have a pending choice from the look-and-select debut ability
    assert!(
        game.has_pending_choice(),
        "bp4 debut (登場) trigger should also fire the look-and-select ability"
    );
}

/// bp4 debut trigger completes look_and_select: select member → hand, rest → waitroom.
#[test]
fn test_bp4_debut_look_select_puts_card_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp4-001-R\u{ff0b}"); // 絢瀬絵里&朝香果林&葉月恋
    let ayumu = game.id("PL!-pb1-011-R"); // 絢瀬絵里 cost=2 — matches select filter
    let filler1 = game.new_id("PL!-sd1-010-SD"); // cost=4 blade=1
    let filler2 = game.new_id("PL!-sd1-010-SD");
    let filler3 = game.new_id("PL!-sd1-010-SD");
    let filler4 = game.new_id("PL!-sd1-010-SD");

    game.add_to_hand(joint);
    // Deck top: [ayumu(matching), filler1, filler2, filler3, filler4, ...]
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(ayumu);
    game.state.player1.main_deck.cards.push(filler1);
    game.state.player1.main_deck.cards.push(filler2);
    game.state.player1.main_deck.cards.push(filler3);
    game.state.player1.main_deck.cards.push(filler4);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
    game.give_energy(20);

    let hand_before = game.state.player1.hand.cards.len();

    // Play joint to stage — this triggers 登場 ability (look top 5, select 1)
    game.play_to_stage(joint, MemberArea::Center);

    // After playing joint to stage, hand decreased by 1
    let hand_after_play = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after_play,
        hand_before - 1,
        "Joint should leave hand when played"
    );

    assert!(
        game.has_pending_choice(),
        "bp4 debut should prompt for card selection from looked-at cards"
    );

    // Select the first card (ayumu — matches name filter)
    game.select_indices(&[0]);

    // After selection: ayumu should be in hand
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        hand_after_play + 1,
        "Selected card should go to hand ({} -> {})",
        hand_after_play,
        hand_after
    );
    assert!(
        game.state.player1.hand.cards.contains(&ayumu),
        "Ayumu should be in hand after selection"
    );

    let waitroom_count = game.state.player1.waitroom.cards.len();
    assert!(
        waitroom_count >= 4,
        "Remaining looked-at cards should go to waitroom (got {})",
        waitroom_count
    );

    // No pending choices should remain
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        !game.has_pending_choice(),
        "All choices should be resolved after look-and-select completes"
    );
}

/// bp4 debut: skip selection → no card to hand, all cards to waitroom.
#[test]
fn test_bp4_debut_skip_select_discards_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp4-001-R\u{ff0b}");
    let ayumu = game.id("PL!-pb1-011-R"); // 絢瀬絵里
    let filler1 = game.new_id("PL!-sd1-010-SD");
    let filler2 = game.new_id("PL!-sd1-010-SD");
    let filler3 = game.new_id("PL!-sd1-010-SD");
    let filler4 = game.new_id("PL!-sd1-010-SD");

    game.add_to_hand(joint);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(ayumu);
    game.state.player1.main_deck.cards.push(filler1);
    game.state.player1.main_deck.cards.push(filler2);
    game.state.player1.main_deck.cards.push(filler3);
    game.state.player1.main_deck.cards.push(filler4);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
        game.state
            .player2
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
    game.state
        .player2
        .hand
        .cards
        .push(game.new_id("PL!-sd1-010-SD"));
    game.give_energy(20);

    game.play_to_stage(joint, MemberArea::Center);
    let hand_after_play = game.state.player1.hand.cards.len();

    assert!(game.has_pending_choice(), "bp4 debut should prompt");

    // Skip — select empty
    game.select_indices(&[]);

    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after, hand_after_play,
        "Skipping selection should not add any card to hand"
    );
    assert!(
        game.state.player1.waitroom.cards.len() >= 5,
        "All looked-at cards should go to waitroom when skipped (got {})",
        game.state.player1.waitroom.cards.len()
    );
}

/// bp4 debut: select matching member → wait opponent members with
/// cost ≤ revealed card's cost AND original blade ≤ 3.
#[test]
fn test_bp4_debut_wait_opponent_members_after_selection() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp4-001-R\u{ff0b}");
    let ayumu = game.id("PL!-pb1-011-R"); // 絢瀬絵里 cost=2, blade=1
    let filler = game.new_id("PL!-sd1-010-SD");

    let p2_low = game.id("PL!-sd1-002-SD"); // cost=2, blade=1 → should wait
    let p2_high_cost = game.id("PL!-sd1-001-SD"); // cost=11, blade=3 → should NOT wait
    let p2_filler = game.id("PL!-sd1-010-SD"); // cost=4, blade=1 → should NOT wait
    game.state.player2.stage.stage = [p2_high_cost, p2_low, p2_filler];

    game.add_to_hand(joint);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(ayumu);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    for _ in 0..10 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
        game.state
            .player2
            .main_deck
            .cards
            .push(game.new_id("PL!-sd1-010-SD"));
    }
    game.state
        .player2
        .hand
        .cards
        .push(game.new_id("PL!-sd1-010-SD"));
    game.give_energy(20);

    game.play_to_stage(joint, MemberArea::Center);
    assert!(game.has_pending_choice(), "bp4 debut should prompt");

    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // ayumu cost=2 → filter: cost≤2 AND original blade≤3
    // p2_low (cost=2, blade=1) should be waited (orientation = "wait")
    let ori = |cid| game.state.mods.get_orientation_modifier(cid);
    assert_eq!(ori(p2_low), Some("wait"), "p2_low should be waited");
    assert_ne!(
        ori(p2_high_cost),
        Some("wait"),
        "p2_high_cost should NOT be waited"
    );
    assert_ne!(
        ori(p2_filler),
        Some("wait"),
        "p2_filler should NOT be waited"
    );
}

// ─────────────────────────────────────────────────────────────
// bp6 — 南ことり＆黒澤ダイヤ＆徒町小鈴
// ライブ開始時: discard ANY NUMBER → gain 1 heart per distinct COLOR of discarded
// ─────────────────────────────────────────────────────────────

/// Discarding 2 cards of 2 different colors → gain 1 of each distinct color.
#[test]
fn test_bp6_live_start_two_distinct_colors_gain_two_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp6-001-R\u{ff0b}"); // 南ことり&黒澤ダイヤ&徒町小鈴
    let kotori = game.id("PL!-bp3-003-R"); // 南ことり — hearts: 01,03,06
    let dia = game.id("PL!S-sd1-004-SD"); // 黒澤ダイヤ — hearts: 02,04,05
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(kotori);
    game.add_to_hand(dia);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(
        game.has_pending_choice(),
        "bp6 should prompt for any-number discard"
    );

    // Discard both cards
    game.try_select_indices(&[0, 1]).unwrap();
    game.select_indices(&[]); // skip re-prompt, finalize

    assert!(
        !game.has_pending_choice(),
        "bp6 should resolve after selection"
    );

    // kotori: heart01, heart03, heart06
    // dia: heart02, heart04, heart05
    // Combined distinct colors: 01+02+03+04+05+06 = 6
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart01),
        1,
        "Heart01 from kotori"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart02),
        1,
        "Heart02 from dia"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart03),
        1,
        "Heart03 from kotori"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart04),
        1,
        "Heart04 from dia"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart05),
        1,
        "Heart05 from dia"
    );
    assert_eq!(
        game.state
            .mods
            .get_heart_modifier(joint, HeartColor::Heart06),
        1,
        "Heart06 from kotori"
    );
}

/// Discarding 2 cards of the SAME color → only 1 heart gained (deduplication).
#[test]
fn test_bp6_live_start_same_color_deduplicates() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp6-001-R\u{ff0b}");
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    // Use two copies of the same card (same colors guaranteed)
    let kotori1 = game.id("PL!-bp3-003-R"); // 南ことり
    let kotori2 = game.new_id("PL!-bp3-003-R"); // another copy, same colors

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(kotori1);
    game.add_to_hand(kotori2);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice(), "bp6 should prompt for discard");

    // Discard both copies
    game.try_select_indices(&[0, 1]).unwrap();
    game.select_indices(&[]); // skip re-prompt, finalize

    // Determine the exact colors from the card.
    // Member cards store their heart colors in base_heart; need_heart is only on live cards.
    let kotori_card = game.db.get_card(kotori1).expect("kotori card must exist");
    let distinct_colors: Vec<HeartColor> = if let Some(ref bh) = kotori_card.base_heart {
        bh.hearts
            .iter()
            .filter(|&&(_, amt)| amt > 0)
            .map(|&(c, _)| c)
            .collect()
    } else {
        vec![]
    };
    let expected: i32 = distinct_colors.len() as i32; // 1 per distinct color, deduped

    let total: i32 = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ]
    .iter()
    .map(|&c| game.state.mods.get_heart_modifier(joint, c))
    .sum();

    assert_eq!(
        total, expected,
        "bp6: 2 same-color cards → {} distinct color(s) → {} heart(s), got {}",
        expected, expected, total
    );
}

/// Skipping the bp6 cost (0 cards) gains no hearts.
#[test]
fn test_bp6_live_start_skip_gains_no_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let joint = game.id("LL-bp6-001-R\u{ff0b}");
    let kotori = game.id("PL!-bp3-003-R");
    let live = game.id("PL!-sd1-010-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = joint;
    game.add_to_hand(kotori);
    game.add_to_hand(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player2.hand.cards.push(filler);
    game.give_energy(10);

    advance_to_live_start(&mut game);
    game.set_live_card(live);
    finish_live_setup(&mut game);

    assert!(game.has_pending_choice(), "bp6 should prompt for discard");

    // Skip — select nothing
    game.select_indices(&[]);

    let total: i32 = [
        HeartColor::Heart01,
        HeartColor::Heart02,
        HeartColor::Heart03,
        HeartColor::Heart04,
        HeartColor::Heart05,
        HeartColor::Heart06,
    ]
    .iter()
    .map(|&c| game.state.mods.get_heart_modifier(joint, c))
    .sum();

    assert_eq!(total, 0, "Skipping bp6 cost should give 0 hearts");
}

// ─────────────────────────────────────────────────────────────
// General: multi-name card identity checks
// ─────────────────────────────────────────────────────────────

/// bp1 card (上原歩夢&澁谷かのん&日野下花帆) must be recognised as having
/// all three individual names (FAQ: the card has all three identities).
#[test]
fn test_joint_card_has_all_three_name_identities() {
    let db = load_real_database();
    let game = TestGame::new(db);

    let joint_id = game.id("LL-bp1-001-R\u{ff0b}");
    let card = game.db.get_card(joint_id).expect("card must exist");
    let names = game.db.get_card_names(joint_id);

    assert!(
        names.iter().any(|n| n.contains("上原歩夢")),
        "Joint card must carry identity '上原歩夢'"
    );
    assert!(
        names.iter().any(|n| n.contains("澁谷かのん")),
        "Joint card must carry identity '澁谷かのん'"
    );
    assert!(
        names.iter().any(|n| n.contains("日野下花帆")),
        "Joint card must carry identity '日野下花帆'"
    );
    assert!(
        card.name.contains('&') || card.name.contains('\u{ff06}'),
        "Card name should contain an ampersand separator"
    );
}

/// The characters filter on a cost matches a multi-name card for ALL constituent names.
#[test]
fn test_multi_name_card_matches_any_constituent_in_characters_filter() {
    use rabuka_engine::ability::util;

    let db = load_real_database();
    let game = TestGame::new(db);

    let joint_id = game.id("LL-bp1-001-R\u{ff0b}");

    // Filter: matches "上原歩夢" — should match the joint card
    let ayumu_names = vec!["上原歩夢".to_string()];
    let filter_ayumu =
        util::filter_from_parts(None, None, None, None, Some(&ayumu_names), None, None);
    assert!(
        filter_ayumu.matches(&game.db, joint_id, false),
        "Joint card should match a filter for '上原歩夢'"
    );

    // Filter: matches "澁谷かのん" — should match the joint card
    let kanon_names = vec!["澁谷かのん".to_string()];
    let filter_kanon =
        util::filter_from_parts(None, None, None, None, Some(&kanon_names), None, None);
    assert!(
        filter_kanon.matches(&game.db, joint_id, false),
        "Joint card should match a filter for '澁谷かのん'"
    );

    // Filter: matches "日野下花帆" — should match the joint card
    let kaho_names = vec!["日野下花帆".to_string()];
    let filter_kaho =
        util::filter_from_parts(None, None, None, None, Some(&kaho_names), None, None);
    assert!(
        filter_kaho.matches(&game.db, joint_id, false),
        "Joint card should match a filter for '日野下花帆'"
    );
}
