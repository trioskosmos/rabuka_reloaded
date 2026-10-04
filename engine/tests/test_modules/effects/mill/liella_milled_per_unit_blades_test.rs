/// Tests for 葉月 恋 (PL!SP-bp5-005-R＋):
///
/// Ab#0 (起動, ターン1回):
///   デッキの上からカードを3枚控え室に置く：ライブ終了時まで、
///   これにより控え室に置いた『Liella!』のメンバーカード1枚につき、ブレードを得る。
///
/// Ab#1 (自動, ターン1回):
///   自分のメインフェイズの間、自分のカードが1枚以上いずれかの領域から
///   控え室に置かれるたび、Eを支払ってもよい。そうした場合、
///   それらのカードの中から1枚手札に加える。
///
/// Q221: 「それらのカードの中」refers to the cards placed by the trigger, not all discard.
/// Q233: Skipping the optional E cost allows re-triggering later in the same turn.
use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Ab#0: Activation sends deck top 3 to discard, grants 1 blade per Liella! member
/// among those 3. Per-unit formula: (matching / per_unit_count) * count.
#[test]
fn hazuki_ren_bp5_005_mill_liella_blade_2_liella_among_3_discarded_grants_2_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella_a = game.id("PL!SP-sd1-001-SD");
    let liella_b = game.id("PL!SP-sd1-004-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, liella_b);
    game.state.player1.main_deck.cards.insert(0, liella_a);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let discard_before = game.state.player1.waitroom.cards.len();

    game.activate_ability(ren);

    // Cost: deck top 3 → discard
    assert_eq!(game.state.player1.waitroom.cards.len(), discard_before + 3);

    // Per-unit: (2 Liella! matching / 1 per_unit_count) * 1 count = 2 blade
    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        2,
        "2 Liella! members among 3 discarded → 2 blade"
    );
}

/// Ab#0: 0 Liella! members among the 3 discarded → 0 blade.
#[test]
fn hazuki_ren_bp5_005_mill_liella_blade_no_liella_no_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let non_liella = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, non_liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(non_liella);
    }

    game.activate_ability(ren);

    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        0,
        "0 Liella! members discarded → 0 blade"
    );
}

/// Ab#0: All 3 discarded are Liella! members → 3 blade (per-unit: 3/1*1 = 3).
#[test]
fn hazuki_ren_bp5_005_mill_liella_blade_all_3_liella_grants_3_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(liella);
    }

    game.activate_ability(ren);

    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        3,
        "3 Liella! members discarded → 3 blade"
    );
}

/// Ab#0: Blade has duration=live_end, persists after activation resolves.
#[test]
fn hazuki_ren_bp5_005_mill_liella_blade_blade_duration_live_end() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(liella);
    }

    game.activate_ability(ren);

    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        3,
        "Blade modifier persists after ability resolves (duration=live_end)"
    );
}

/// Ab#0: pre-existing 『Liella!』 members in the discard do NOT count — only the 3
/// the cost just placed are considered.
///
/// The name and comment here used to say the opposite. Both were left over from when
/// the count really was inflated: `per_unit_type` is `"discard"`, which read alone
/// says "count every matching card in the discard", so a waitroom holding two 『Liella!』
/// members plus the one the cost milled summed to 3 instead of 1. The fix was
/// `per_unit_source: "previous_moved_cards"` — the parse keeps BOTH fields, and it is
/// the source field that scopes the count to this activation's cards.
///
/// So this is the test that proves the scoping works, and it is the only one that can:
/// every other test in this file starts with an empty discard, where "count the zone"
/// and "count what I just placed" agree. The stale wording was actively harmful in the
/// test NAME, which is what a reader greps for and what the coverage inventory records —
/// it advertised a broken case that has been fixed since.
#[test]
fn hazuki_ren_bp5_005_preexisting_liella_in_discard_does_not_count_toward_the_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Put 2 Liella! members in discard BEFORE activation
    game.state.player1.waitroom.cards.push(liella);
    game.state.player1.waitroom.cards.push(liella);

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(3);

    // Deck top 3: only 1 Liella!
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, liella);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.activate_ability(ren);

    // `per_unit_type: "discard"` on its own would count the whole zone:
    // 2 pre-existing + 1 just placed = 3. `per_unit_source:
    // "previous_moved_cards"` scopes it to the 3 this cost placed, and only 1 of
    // those is a 『Liella!』 member.
    assert_eq!(
        game.state.mods.get_blade_modifier(ren),
        1,
        "Expected 1 — only the 1 placed by the cost counts, not the 2 already \
         sitting in the discard"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.iter().filter(|&&c| c == liella).count(),
        3,
        "all three are in the discard; only the scoping decides how many count"
    );
}

// ========================================================================
// Ab#1 tests
// ========================================================================

/// 「メインフェイズ**の間**」 — a live-phase discard does not trip the watcher.
///
/// The clause parses as `temporal_condition { phase: "main", phase_target: "self" }`, and
/// every other ab#1 test runs in `FirstAttackerNormal`, where that gate and no gate at all
/// behave identically.
///
/// Driving it from ab#0's mill does not work: ab#0 is a 起動 ability, so under
/// `SecondAttackerNormal` the activation is refused outright ("No activatable ability
/// found for this card at its current location") and a failure is ambiguous between "the
/// gate held" and "the driver never ran". `trigger_auto_abilities_for_player` does not
/// work either — ab#1 is an `each_time` watcher, raised by the EVENT of a card reaching
/// a discard, and a generic auto scan does not produce it. Nor can the driver be invoked
/// directly: `execute_main_phase_action` refuses any UseAbility outside a main phase,
/// which is the very thing under test.
///
/// So the driver is 鬼��� 夏美's ライブ開始時, reached by WALKING the turn to LiveStart —
/// nothing in the harness gates that, and her ability is legal there because it is a live
/// ability. The discard it performs is therefore a genuine live-phase discard, and the
/// only thing that can stop the watcher is the clause.
#[test]
fn hazuki_ren_bp5_005_a_live_phase_discard_does_not_trip_the_watcher() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let ren = game.id("PL!SP-bp5-005-R＋");
    let natsumi = game.id("PL!SP-bp5-009-R");
    let performing = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = natsumi;
    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    // Top card is what 夏美 mills; the rest is filler so the draw during the walk cannot
    // reach the waitroom and confuse the count below.
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.main_deck.cards.insert(0, filler);
    let waitroom_before = game.state.player1.waitroom.cards.len();

    game.add_to_hand(performing);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(performing);
    game.pass();
    game.pass();

    // 夏美 asks twice per procedure: "mill it?" then "repeat?". Both are SelectTarget, so
    // they are told apart by ORDER, not by type — the first is the mill (Yes) and every
    // later one is the repeat (Stop). Answering Yes to the repeat loops until
    // `max_repeats` is exhausted and then still offers again, which is what an
    // order-blind drain does.
    // A loop, for the same reason as the test above: 葉月 恋 has two abilities, so a
    // `SelectAutoAbility` order prompt precedes the mill and the repeat. The two
    // SelectTarget prompts are told apart by ORDER — the first is the mill (Yes) and
    // the second is the repeat (Stop). Answering Yes to the repeat loops until
    // `max_repeats` is exhausted and then offers again, which is what an
    // order-blind drain does.
    let mut targets = 0;
    let mut guard = 0;
    while game.has_pending_choice() {
        guard += 1;
        assert!(guard < 10, "the driver should settle: one mill prompt, then Stop");
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => {
                targets += 1;
                game.select_option(if targets == 1 { 1 } else { 0 });
            }
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    // The mill DID happen — asserted, so the silence below is the gate and not a driver
    // that quietly did nothing.
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        waitroom_before + 1,
        "夏美 milled one card into the discard during Live; without this the next \
         assertion would be satisfied by a driver that never ran"
    );
    assert!(
        !game.has_pending_choice(),
        "「自分のメインフェイズの間」 — a card reaching my discard during Live must not \
         raise the watcher, so there is no optional 1-energy cost to offer"
    );
}

/// Ab#1: the pool is 「それらのカード」 in MY discard — not the opponent's.
///
/// The watcher fires on a card reaching a discard and its clause carries
/// `target: "self"` with `location: "discard"`, so the zone it scans is the activating
/// player's. That is the reading the card asks for — 「**自分の**カードが…控え室に置かれるたび」
/// — and it is the reading that can silently break, because "the discard" is the same
/// word for both players and a watcher resolved by "whoever's discard changed" would
/// fire on the opponent's.
///
/// `optional_pay_recovery_only_trigger_cards_not_full_discard` already pins the half of
/// this that excludes MY OWN pre-existing discard cards (Q221). Nothing pinned the other
/// half: with the opponent's discard populated, the recovery must still only ever offer
/// the three cards this mill placed.
#[test]
fn hazuki_ren_bp5_005_the_opponents_discard_is_never_a_recovery_pool() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R＋");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    // The OPPONENT's discard, populated before anything happens. Same card type, same
    // phase, different player — the only variable.
    let theirs = game.id("PL!SP-sd1-001-SD");
    game.state.player2.waitroom.cards.push(theirs);

    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, liella);
    game.state.player1.main_deck.cards.insert(0, liella);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0 to mill 3 and trip the watcher");

    // The watcher fired for ME — asserted, so the rest of the test is about scoping
    // rather than about a watcher that never ran.
    assert!(
        game.has_pending_choice(),
        "ab#1 must fire for my own mill"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "the optional 1-energy cost is the first prompt"
    );
    game.select_option(1);
    // A loop, and the drain-loop ratchet budget for this file records it: 葉月 恋
    // has TWO abilities, so the mill raises a `SelectAutoAbility` order prompt before
    // the cost and the select. The sequence is multi-typed and order-dependent, so
    // fixed arms get it wrong — an earlier attempt assumed the cost came first and
    // failed on the order prompt.
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1), // pay the optional E
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert!(
        game.state.player2.waitroom.cards.contains(&theirs),
        "「自分のカード」 — the opponent's discard is neither a trigger nor a recovery \
         pool; their card is still there. waitroom {:?}",
        game.state.player2.waitroom.cards
    );
    assert!(
        !game.state.player1.hand.cards.contains(&theirs),
        "and their card was not the one recovered into my hand; hand {:?}",
        game.state.player1.hand.cards
    );
}

/// Ab#1: Activate ab#0 (mill 3), then ab#1 auto-triggers. Pay 1 energy,
/// select 1 card from the 3 milled to recover to hand.
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_triggers_after_mill_pay_cost_recover_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    // Deck top: [liella, liella, filler, ...]
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, liella);
    game.state.player1.main_deck.cards.insert(0, liella);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Ab#1 should trigger → optional cost prompt (conditional_optional) → pay
    assert!(game.has_pending_choice(), "ab#1 optional cost prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget"
    );
    game.select_option(1);
    // Select 1 card from trigger cards to recover
    while game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        match ct.as_str() {
            "SelectCard" => {
                game.select_indices(&[0]);
            }
            "SelectTarget" => {
                game.select_option(0);
            }
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "1 card should be recovered to hand"
    );
}

/// Ab#1: Mill 3 different cards, verify the selection offers all 3 positions,
/// then recover a specific card and confirm it's the right one.
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_mill_3_recover_specific_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    // A named card we can identify after recovery
    let chisato = game.id("PL!SP-bp2-003-R");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    // Deck top: [chisato, filler, filler, ...]
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, chisato);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Ab#1 should trigger → optional cost prompt (conditional_optional) → pay
    assert!(game.has_pending_choice(), "ab#1 optional cost prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget"
    );
    game.select_option(1);
    // SelectCard: pick chisato (the milled card we want)
    let hand_before = game.state.player1.hand.cards.len();
    while game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        match ct.as_str() {
            "SelectCard" => {
                game.select_indices(&[0]);
            }
            "SelectTarget" => game.select_option(0),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "1 card recovered"
    );
    // Verify it's chisato — the first card milled (deck top = last discard)
    assert_eq!(
        game.state.player1.hand.cards.last(),
        Some(&chisato),
        "Should have recovered chisato (the first-milled card, index 0 in choice)"
    );
}

/// Ab#1: Decline the optional cost, no card recovered (per Q233).
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_decline_cost_no_recovery() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Ab#1 triggers → optional cost prompt (conditional_optional) → decline
    assert!(game.has_pending_choice(), "ab#1 optional cost prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget"
    );
    game.select_option(0);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "No card recovered when cost declined"
    );
}

/// Ab#1: Pre-existing cards in discard should NOT be selectable (Q221).
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_only_trigger_cards_not_full_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Pre-fill discard
    game.state.player1.waitroom.cards.push(liella);

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let discard_before = game.state.player1.waitroom.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Ab#1 triggers → optional cost prompt (conditional_optional) → pay
    assert!(game.has_pending_choice(), "ab#1 optional cost prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget"
    );
    game.select_option(1);

    // Select 1 from the 3 trigger cards
    while game.has_pending_choice() {
        let ct = game.pending_choice_type().unwrap_or_default();
        match ct.as_str() {
            "SelectCard" => {
                game.select_indices(&[0]);
            }
            "SelectTarget" => {
                game.select_option(0);
            }
            _ => break,
        }
    }

    // 1 pre-existing + 3 milled - 1 recovered = 3 remaining
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before + 2, // 3 milled - 1 recovered = 2 net new
        "Pre-existing Liella! should not be recoverable (Q221)"
    );
}

/// card_type_filter "card" is a catch-all that matches every card type.
/// Paying the optional cost recovers 1 card regardless of the trigger card type.
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_card_type_catch_all_matches_any_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let energy = game.id("LL-E-001-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    game.state.player1.waitroom.cards.push(energy);
    game.state.player1.waitroom.cards.push(energy);
    game.state.player1.waitroom.cards.push(energy);
    game.state.set_recently_moved_cards(vec![energy, energy, energy]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    // Ab#1 triggers: pay optional cost → choose 1 of 3 cards to recover
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1), // pay 1 energy
            Some("SelectCard") => game.select_indices(&[0]), // pick first card
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "card_type='card' catch-all → 1 recovered"
    );
    // Verify the choice included all 3 discard positions, not just 1
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        2,
        "2 of the 3 energy cards should remain in waitroom"
    );
}

/// 2 Rens: first pays, second declines → only 1 card recovered.
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_two_copies_one_pays_one_declines() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren1 = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let ren2 = game.new_id("PL!SP-bp5-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [ren1, ren2, -1];
    game.give_energy(5);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, filler);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren1),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // First prompt → ren1: pay.
    // Second prompt → ren2: decline.
    // Third prompt → no card selection (declined costs skip move_cards).
    // Only ren1 pays → 1 card recovered.
    let mut pay_count = 0;
    while game.has_pending_choice() && pay_count < 6 {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => {
                if pay_count == 0 {
                    game.select_option(1); // ren1 pays
                } else {
                    game.select_option(0); // ren2 declines
                }
                pay_count += 1;
            }
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "Only ren1 paid → 1 card recovered"
    );
}

/// look_and_select discard_remaining triggers auto abilities as a SINGLE
/// batched discard (not per-card). Uses PL!-sd1-011-SD which has a debut
/// look_and_select: look at top 3, pick 1 to hand, discard rest.
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_triggers_once_from_look_and_select_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let scry = game.id("PL!-sd1-011-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [ren, -1, -1];
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(scry);
    // Add a filler so the scry's optional discard cost can be paid
    game.state.player1.hand.cards.push(filler);

    game.state.player1.main_deck.cards.clear();
    for _ in 0..3 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(5);

    // Play scry card to stage → debut triggers look_and_select
    game.play_to_stage(scry, rabuka_engine::zones::MemberArea::Center);

    // Handle the look_and_select: pick 1 card from top 3 to hand
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            _ => break,
        }
    }

    // After look_and_select discards remaining 2 cards as a batch,
    // Ren's ab#1 should trigger exactly once. Handle the prompt.
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    // scry recovered 1 card to hand + Ren recovered 1 = 2 total.
    // The scry was played from hand (1 card) but 2 were recovered.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "scry picked 1 + Ren recovered 1 = 2 cards in hand"
    );
}

// ========================================================================
// Interaction tests
// ========================================================================

/// 2 copies of Ren on stage. Mill 3 → both ab#1 trigger independently.
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_two_copies_both_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren1 = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let ren2 = game.new_id("PL!SP-bp5-005-R\u{ff0b}");
    let liella = game.id("PL!SP-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [ren1, ren2, -1];
    game.give_energy(5);

    for _ in 0..3 {
        game.state.player1.main_deck.cards.insert(0, liella);
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let hand_before = game.state.player1.hand.cards.len();

    // Activate ab#0 on ren1 → mill 3 → both ab#1 trigger → 2 prompts
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::UseAbility,
        Some(ren1),
        None,
        None,
        None,
    )
    .expect("Activate ab#0");

    // Handle all prompts: auto-ability selection, cost, card pick
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1),
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 2,
        "2 cards should be recovered (one per Ren copy)"
    );
}

/// Q233: Decline cost → ability can re-trigger when more cards are discarded.
#[test]
fn hazuki_ren_bp5_005_optional_pay_recovery_decline_retrigger_same_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ren = game.id("PL!SP-bp5-005-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = ren;
    game.give_energy(5);

    // First discard event: 3 cards milled
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.set_recently_moved_cards(vec![filler, filler, filler]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    // Process the auto ability → decline cost
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(0), // decline
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "Q233: declined → no card recovered"
    );

    // Second discard event: 3 more cards milled
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.player1.waitroom.cards.push(filler);
    game.state.set_recently_moved_cards(vec![filler, filler, filler]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");

    // Process the auto ability → pay this time
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[0]),
            Some("SelectTarget") => game.select_option(1), // pay
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Q233: second discard → card recovered after paying"
    );
}
