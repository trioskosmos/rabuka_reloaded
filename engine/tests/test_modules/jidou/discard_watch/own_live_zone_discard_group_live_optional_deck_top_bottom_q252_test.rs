use crate::helpers::*;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

fn trigger_ability(game: &mut TestGame, card_id: i16, trigger_str: &str) {
    let card = game.db.get_card(card_id).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some(trigger_str))
        .unwrap();
    let pid = game.state.player1.id.clone();
    let trigger = match trigger_str {
        "登場" => rabuka_engine::core::types::AbilityTrigger::Debut,
        "ライブ開始時" => rabuka_engine::core::types::AbilityTrigger::LiveStart,
        "起動" => rabuka_engine::core::types::AbilityTrigger::Activation,
        _ => rabuka_engine::core::types::AbilityTrigger::Auto,
    };
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
}

// =========================================================================
// PL!S-bp6-002-SEC — 桜内梨子 (Sakurauchi Riko)
// Ability 0 (自動 ターン1回):
//   『Aqours』のライブカードが自分のライブカード置き場から控え室に置かれたとき、
//   そのライブカードをデッキの一番上か一番下に置いてもよい。
//
// PARSING BUG: condition uses locations:["live_card_zone","discard"] as a STATIC
//   multi-zone membership check, not a movement trigger. The effect's source is
//   also empty → defaults to "discard" (not the specific moved card).
// =========================================================================

/// FIXED: condition uses source:"preceding_moved" so only cards that actually
/// moved from live_card_zone trigger the ability. A card just sitting in discard
/// with no movement history does NOT trigger.
#[test]
fn own_live_zone_discard_optional_deck_does_not_fire_without_live_card_zone_movement() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR"); // Aqours live

    game.state.player1.stage.stage = [-1, riko, -1];
    // Card was never in live_card_zone — directly placed in discard, no movement
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, riko, "自動");

    // FIXED: condition uses preceding_moved — no recently_moved cards → no trigger
    assert!(
        !game.has_pending_choice(),
        "Fixed: no movement → no trigger"
    );
}

// ====================================================================
// FULL GAME FLOW tests — Riko BP6 auto ability fires naturally when
// live cards move from live_card_zone → waitroom during performance.
// ====================================================================

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Heart failure: MIRACLE WAVE needs h02=4, h04=4, h05=4 (total 12).
/// Riko provides h02=2, h04=2, h05=2 (total 6) — not enough → all cards
/// go to waitroom. Riko's auto ability should fire and offer to put the
/// Aqours live card on deck.
#[test]
fn own_live_zone_discard_optional_deck_e2e_heart_failure_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let mw = game.id("PL!S-bp3-019-L"); // Aqours, need 12 hearts

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.hand.cards.push(mw);

    // Fill deck for yell draws
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(mw);

    // Drive the FULL round: P1 fails MIRACLE WAVE's 12-heart requirement →
    // lives go to waitroom → Riko's auto fires and offers the optional
    // deck-top/bottom placement → ACCEPT it ([0]) so she relocates the live.
    for _ in 0..8 {
        while game.has_pending_choice() {
            game.select_indices(&[0]);
        }
        game.pass();
    }
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Riko's effect moved the failed live onto her deck (top); later natural
    // draws in the loop may pull it into hand — either way it must NOT be
    // back in the waitroom.
    assert!(
        !game.state.player1.waitroom.cards.contains(&mw),
        "failed live must leave the waitroom via Riko BP6 auto"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&mw)
            || game.state.player1.hand.cards.contains(&mw),
        "relocated live ends up in deck or (after draws) hand"
    );
}

/// Heart success: HAPPY PARTY TRAIN needs h02=1, h04=1, h05=1 (total 3).
/// Riko provides h02=2, h04=2, h05=2 (total 6) — enough → card stays in
/// live_card_zone / success zone. Riko's auto ability should NOT fire.
#[test]
fn own_live_zone_discard_optional_deck_e2e_heart_success_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let hpt = game.id("PL!S-PR-022-PR"); // Aqours, need 3 hearts

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.hand.cards.push(hpt);

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(hpt);

    while game.has_pending_choice() {
        assert!(
            game.pending_choice_type().is_some(),
            "live-setup prompt must carry a choice identity"
        );
        game.select_indices(&[]);
    }

    // Advance through performance phases
    game.pass(); // FirstAttackerPerformance
    game.pass(); // SecondAttackerPerformance
    game.pass(); // LiveVictoryDetermination
    game.pass(); // → Active

    // The live card should NOT be in waitroom (it passed → success zone or stayed)
    // Riko's auto ability should NOT have fired (no card moved to waitroom)
    // Verify by checking there's no pending choice from Riko
    let has_riko_choice = game.has_pending_choice();
    assert!(
        !has_riko_choice,
        "Riko BP6 auto ability should NOT fire when live card passes"
    );
}

/// cannot_live: Player is marked as cannot_live → all live cards go to
/// waitroom during yell phase. Riko's auto ability should fire and move
/// the card to deck.
#[test]
fn own_live_zone_discard_optional_deck_e2e_cannot_live_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR"); // Aqours live

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.hand.cards.push(live);
    game.state.cannot_live_players.push("p1".into());

    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);

    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();

    // Advance to FirstAttackerPerformance — cannot_live moves cards to waitroom,
    // then 8.3.13 check timing fires auto abilities. Riko should trigger.
    game.pass();

    // After the performance, the live card should NOT be in waitroom —
    // Riko's auto ability moved it to deck. If it's still in waitroom,
    // the trigger didn't fire.
    let in_waitroom = game.state.player1.waitroom.cards.contains(&live);
    assert!(
        !in_waitroom,
        "cannot_live: Riko BP6 should have moved the live card from waitroom to deck"
    );

    // Handle any remaining pending choices (position selection etc.)
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectPosition") => {
                game.select_indices(&[0]);
            }
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            _ => break,
        }
    }
}

/// Aqours live card in live_card_zone only (not in discard) — condition passes
/// but effect finds nothing to move (source defaults to discard). No choice.
#[test]
fn own_live_zone_discard_optional_deck_live_card_only_in_zone_no_discard_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.live_card_zone.cards.push(live);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, riko, "自動");

    // No card in discard → effect source is empty → no cards moved → no choice
    // CORRECT behavior would be: trigger only when card moves, and move THAT card
    assert!(
        !game.has_pending_choice(),
        "No card in discard → effect finds nothing → no choice"
    );
}

#[test]
fn own_live_zone_discard_optional_deck_no_live_cards_anywhere_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);
    game.give_energy(5);

    trigger_ability(&mut game, riko, "自動");

    assert!(
        !game.has_pending_choice(),
        "No live cards → condition fails"
    );
}

// =========================================================================
// PL!S-bp6-002-SEC — 桜内梨子 (Riko) Ability 0 full flow tests
// Using the REAL engine path: trigger_auto_abilities_for_player scans the
// stage for auto abilities, enqueues with trigger_moved_cards captured from
// recently_moved_cards, then processes the queue naturally.
// No injected functions — every choice goes through generate_possible_actions
// + select_generated, exactly like the game server.
// =========================================================================

fn trigger_riko_auto(game: &mut TestGame, moved_cards: Vec<i16>) {
    game.state.set_recently_moved_cards(moved_cards);
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());
}

/// Single Aqours live card moved → ability fires → position|destination choice →
/// select_generated(0) for deck_top → card placed on top of deck, waitroom has 0 copies.
#[test]
fn own_live_zone_discard_optional_deck_single_card_deck_top_exact_identity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);

    let deck_snapshot: smallvec::SmallVec<[i16; 64]> = game.state.player1.main_deck.cards.clone();

    trigger_riko_auto(&mut game, vec![live]);

    assert!(
        game.has_pending_choice(),
        "position|destination choice must appear"
    );
    game.select_generated(0);

    // live removed from waitroom entirely
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "target live card removed from waitroom"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        0,
        "no cards remain in waitroom"
    );
    // deck: original + live on top
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_snapshot.len() + 1,
        "deck gained exactly 1 card"
    );
    assert_eq!(
        game.state.player1.main_deck.cards[0], live,
        "live card is at deck position 0 (top)"
    );
    // Check deck order below top preserved — use Vec comparison
    let expected_tail: Vec<i16> = deck_snapshot.iter().copied().collect();
    let actual_tail: Vec<i16> = game.state.player1.main_deck.cards[1..].to_vec();
    assert_eq!(
        actual_tail, expected_tail,
        "original deck order preserved below top"
    );
}

/// Same setup but select_generated(1) for deck_bottom — card at last position.
#[test]
fn own_live_zone_discard_optional_deck_single_card_deck_bottom_exact_identity() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);

    let deck_snapshot: smallvec::SmallVec<[i16; 64]> = game.state.player1.main_deck.cards.clone();

    trigger_riko_auto(&mut game, vec![live]);

    assert!(
        game.has_pending_choice(),
        "position|destination choice must appear"
    );
    game.select_generated(1); // deck_bottom

    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "target live card removed from waitroom"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_snapshot.len() + 1,
        "deck gained exactly 1 card"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.last(),
        Some(&live),
        "live card is at deck bottom"
    );
    let snapshot_vec: Vec<i16> = deck_snapshot.iter().copied().collect();
    let actual_head: Vec<i16> = game.state.player1.main_deck.cards[..deck_snapshot.len()].to_vec();
    assert_eq!(
        actual_head, snapshot_vec,
        "original deck order preserved above bottom"
    );
}

/// 3 Aqours live cards moved in one batch → ONLY the first in trigger_moved_cards
/// order gets taken. The other 2 stay. One shot, no per-card prompts.
/// This is the realistic max (3 live slots). Verifies the auto fires ONCE per
/// trigger event and takes `count: 1` from the ordered batch.
#[test]
fn own_live_zone_discard_optional_deck_batch_of_3_first_only_taken_others_untouched() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    // 3 distinct Aqours live cards (max live zone capacity)
    let batch: Vec<i16> = (0..3).map(|_| game.id("PL!S-PR-022-PR")).collect();

    game.state.player1.stage.stage = [-1, riko, -1];
    for &c in &batch {
        game.state.player1.waitroom.cards.push(c);
    }
    fill_decks(&mut game, filler);
    let deck_before = game.state.player1.main_deck.cards.len();

    trigger_riko_auto(&mut game, batch.clone());

    // Q252: first a SelectCard choice appears for the player to pick which card.
    assert!(
        game.has_pending_choice(),
        "SelectCard choice expected (Q252: pick 1 from multiple)"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "first choice should be card selection"
    );
    // Pick the second card in the batch (index 1 in the generated list)
    game.select_indices(&[1]);

    // Then position|destination choice for deck top or bottom
    assert!(
        game.has_pending_choice(),
        "position|destination choice must appear after card selection"
    );
    game.select_generated(0); // deck_top

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before + 1,
        "exactly 1 card added to deck"
    );
    // The player picked index 1 (batch[1]), which is the second card
    let on_deck = game.state.player1.main_deck.cards[0];
    assert_eq!(on_deck, batch[1], "player picked the second card (index 1)");

    // The other 2 remain in waitroom
    let waitroom = &game.state.player1.waitroom.cards;
    assert_eq!(waitroom.len(), 2, "2 cards remain in waitroom");
    assert!(
        waitroom.contains(&batch[0]),
        "batch[0] remains in waitroom (not picked)"
    );
    assert!(
        waitroom.contains(&batch[2]),
        "batch[2] remains in waitroom (not picked)"
    );
    // The picked card batch[1] is NOT in waitroom
    assert!(
        !waitroom.contains(&batch[1]),
        "batch[1] was picked and removed from waitroom"
    );
}

/// Mixed batch: 1 Aqours live + 1 non-Aqours live + 1 member card + 1 energy.
/// The condition filters to Aqours+live, so only 1 candidate. That card is taken.
#[test]
fn own_live_zone_discard_optional_deck_mixed_batch_filters_correctly() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let aq_live = game.id("PL!S-PR-022-PR");
    let non_aq_live = game.id("PL!-bp5-019-L"); // μ's, not Aqours
    let filler_member = game.id("PL!-sd1-010-SD");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(aq_live);
    game.state.player1.waitroom.cards.push(non_aq_live);
    game.state.player1.waitroom.cards.push(filler_member);
    game.state.player1.waitroom.cards.push(energy);
    fill_decks(&mut game, filler);

    // Put aq_live first in trigger order
    trigger_riko_auto(&mut game, vec![aq_live, non_aq_live, filler_member, energy]);

    assert!(
        game.has_pending_choice(),
        "condition passes — at least 1 Aqours live moved"
    );
    game.select_generated(0);

    // Only aq_live was removed; others untouched
    assert!(
        !game.state.player1.waitroom.cards.contains(&aq_live),
        "aq_live removed"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_aq_live),
        "non-Aqours live remains"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&filler_member),
        "member card remains"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&energy),
        "energy card remains"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&aq_live),
        "aq_live on deck top"
    );
}

/// Non-Aqours live card alone: condition counts 0 matching → no trigger.
#[test]
fn own_live_zone_discard_optional_deck_non_aq_live_alone_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let non_aq = game.id("PL!-bp5-019-L");

    game.state.player1.stage.stage = [-1, riko, -1];
    game.state.player1.waitroom.cards.push(non_aq);
    fill_decks(&mut game, filler);

    trigger_riko_auto(&mut game, vec![non_aq]);

    assert!(
        !game.has_pending_choice(),
        "non-Aqours live → condition fails"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_aq),
        "non-Aqours live stays in waitroom"
    );
}

/// First trigger succeeds (deck top). Second trigger in same turn hits
/// the turn limit (ターン1回) and produces NO choice. The second card
/// sits untouched in waitroom. Energy and recently_moved_cards are set
/// up identically both times.
#[test]
fn own_live_zone_discard_optional_deck_turn_limit_blocks_second_trigger_exact() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!S-PR-022-PR");
    let live_b = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    // --- First trigger ---
    game.state.player1.waitroom.cards.push(live_a);
    trigger_riko_auto(&mut game, vec![live_a]);
    assert!(game.has_pending_choice(), "first trigger: choice appears");
    game.select_generated(0);
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_a),
        "first trigger: live_a moved to deck"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live_a),
        "first trigger: live_a on deck top"
    );

    // --- Second trigger (same turn) ---
    game.state.player1.waitroom.cards.push(live_b);
    game.state.set_recently_moved_cards(vec![live_b]);
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());

    assert!(
        !game.has_pending_choice(),
        "second trigger: turn limit — no choice"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_b),
        "second trigger: live_b remains in waitroom"
    );
    // live_a still on deck top (untouched by second trigger)
    assert_eq!(
        game.state.player1.main_deck.cards.first(),
        Some(&live_a),
        "second trigger: live_a still on deck top"
    );
}

/// Edge: trigger_moved_cards is empty vec — no cards were actually moved.
/// Condition counts 0 matching → no trigger, no choice.
#[test]
fn own_live_zone_discard_optional_deck_empty_moved_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    trigger_riko_auto(&mut game, vec![]);

    assert!(
        !game.has_pending_choice(),
        "empty moved_cards → condition fails"
    );
}

/// Edge: trigger_moved_cards is None (not set at all) — fallback to
/// recently_moved_cards which is also None → condition fails.
#[test]
fn own_live_zone_discard_optional_deck_null_moved_cards_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    // Don't set recently_moved_cards at all
    game.state
        .trigger_auto_abilities_for_player(&game.state.player1.id.clone());
    game.state
        .process_pending_auto_abilities(&game.state.player1.id.clone());

    assert!(
        !game.has_pending_choice(),
        "null moved_cards → condition fails"
    );
}

/// Riko not on stage — trigger_auto_abilities_for_player scans stage
/// cards for AUTO triggers. Riko isn't there → nothing enqueued.
#[test]
fn own_live_zone_discard_optional_deck_watcher_off_stage_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let _riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!S-PR-022-PR");

    // Riko NOT on stage — empty stage
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.player1.waitroom.cards.push(live);
    fill_decks(&mut game, filler);

    trigger_riko_auto(&mut game, vec![live]);

    assert!(
        !game.has_pending_choice(),
        "riko off stage → no auto trigger"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "live card stays in waitroom"
    );
}

/// Optional skip: per QA Q252, the turn limit is consumed by the EFFECT
/// (placing on deck), not the trigger. Choosing skip leaves the card in
/// waitroom and DOES NOT consume the turn limit — the ability can fire
/// again in the same turn when another Aqours live card moves.
#[test]
fn own_live_zone_discard_optional_deck_optional_skip_does_not_consume_turn_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let riko = game.id("PL!S-bp6-002-SEC");
    let filler = game.id("PL!-sd1-010-SD");
    let live_a = game.id("PL!S-PR-022-PR");
    let live_b = game.id("PL!S-sd1-019-SD");

    game.state.player1.stage.stage = [-1, riko, -1];
    fill_decks(&mut game, filler);

    // --- First trigger: skip the optional effect ---
    game.state.player1.waitroom.cards.push(live_a);
    trigger_riko_auto(&mut game, vec![live_a]);
    assert!(
        game.has_pending_choice(),
        "first trigger: position|destination choice appears"
    );
    game.select_option(-1); // skip
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a),
        "skip: live_a stays in waitroom"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&live_a),
        "skip: live_a not on deck"
    );

    // --- Second trigger (same turn): skip did NOT consume turn limit ---
    game.state.player1.waitroom.cards.push(live_b);
    trigger_riko_auto(&mut game, vec![live_b]);
    assert!(
        game.has_pending_choice(),
        "second trigger: skip didn't consume turn limit — choice appears"
    );
    game.select_generated(0); // deck_top
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_b),
        "live_b moved to deck from second trigger"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&live_b),
        "live_b on deck"
    );
    // live_a still in waitroom (from the skip)
    assert!(
        game.state.player1.waitroom.cards.contains(&live_a),
        "live_a still in waitroom (was skipped)"
    );

    // --- Third trigger: now the turn limit IS consumed (live_b placed on deck) ---
    let live_c = game.id("PL!S-PR-022-PR");
    game.state.player1.waitroom.cards.push(live_c);
    trigger_riko_auto(&mut game, vec![live_c]);
    assert!(
        !game.has_pending_choice(),
        "third trigger: turn limit consumed by live_b placement — no choice"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&live_c),
        "live_c stays in waitroom (turn limit used)"
    );
}


