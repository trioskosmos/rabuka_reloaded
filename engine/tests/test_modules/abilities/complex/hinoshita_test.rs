/// Tests for 日野下花帆 (PL!HS-bp5-001-R+) — Activation ability (ab#1):
///
/// {{kidou.png|起動}}{{turn1.png|ターン1回}}{{icon_energy.png|E}}{{icon_energy.png|E}}
/// 手札のライブカードを1枚公開する：自分の控え室から、これにより公開したカードの
/// カード名がすべて含まれるライブカードを1枚手札に加える。
///
/// Q236: Reveal "Dream Believers" → can recover "Dream Believers (104th Ver.)"
///       because the fragment "Dream Believers" is contained in the target name.
/// Q237: Reveal "Dream Believers (104th Ver.)" → CANNOT recover "Dream Believers"
///       because the fragment "(104th Ver.)" is NOT contained in "Dream Believers".
///
/// Cost breakdown:
///   1) Pay 2 energy (auto-deducted from energy zone)
///   2) Reveal 1 live card from hand (card stays in hand, name used as search key)
///
/// Effect: Move 1 live card from waitroom whose name CONTAINS ALL fragments
///         of the revealed card's name (split by '&') into hand.
///
/// Note: When exactly 1 matching card exists in hand for step 2 or in
///       waitroom for the effect, no choice is prompted (auto-select).
///       When multiple candidates exist, a selection choice is created.
use crate::helpers::*;

/// Q236: Reveal "Dream Believers" (short name) → CAN recover
/// "Dream Believers (104th Ver.)" because the fragment "Dream Believers"
/// from the revealed card IS contained in the target's name.
///
/// Verifies:
///   - Revealed card stays in hand (NOT discarded)
///   - Target card moves from waitroom to hand
///   - 2 energy consumed
#[test]
fn hinoshita_q236_name_contains_all_matches() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_live = game.id("PL!HS-bp1-019-L"); // "Dream Believers" — base name
    let target_live = game.id("PL!HS-bp5-017-L"); // "Dream Believers (104th Ver.)" — variant

    // Stage: 日野下花帆 at center
    game.state.player1.stage.stage[1] = hino;

    // Hand: the live card to reveal
    game.state.player1.hand.cards.push(reveal_live);

    // Waitroom: the live card to recover
    game.state.player1.waitroom.cards.push(target_live);

    // Energy: 2 for activation cost
    game.give_energy(2);
    let energy_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(hino);

    // Exactly 1 live card in hand → auto-select (no choice for reveal).
    // Exactly 1 matching card in waitroom → auto-select (no choice for recovery).
    // So has_pending_choice() should be false.
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Q236: Target should be in hand
    assert!(
        game.state.player1.hand.cards.contains(&target_live),
        "Q236: Should recover Dream Believers (104th Ver.) — fragments 'Dream Believers' match"
    );

    // Revealed card must stay in hand (公開する = reveal, NOT discard)
    assert!(
        game.state.player1.hand.cards.contains(&reveal_live),
        "Q236: Revealed card must stay in hand — '公開する' reveals without discarding"
    );

    // 2 energy consumed
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before - 2,
        "Q236: 2 energy should be consumed"
    );
}

/// Q237: Reveal "Dream Believers (104th Ver.)" (longer name) → CANNOT recover
/// "Dream Believers" because the extra fragment "(104th Ver.)" from the
/// revealed card is NOT contained in the target's name.
///
/// Verifies:
///   - Revealed card stays in hand
///   - Target card stays in waitroom (not moved)
///   - 2 energy consumed
#[test]
fn hinoshita_q237_name_contains_all_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_live = game.id("PL!HS-bp5-017-L"); // "Dream Believers (104th Ver.)" — has extra text
    let target_live = game.id("PL!HS-bp1-019-L"); // "Dream Believers" — base name, no extra text

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal_live);
    game.state.player1.waitroom.cards.push(target_live);
    game.give_energy(2);
    let energy_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(hino);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Q237: Target should NOT be recoverable
    assert!(
        !game.state.player1.hand.cards.contains(&target_live),
        "Q237: Should NOT recover Dream Believers — extra fragment '(104th Ver.)' not contained in target name"
    );

    // Target stays in waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&target_live),
        "Q237: Target card should remain in waitroom"
    );

    // Revealed card must stay in hand
    assert!(
        game.state.player1.hand.cards.contains(&reveal_live),
        "Q237: Revealed card must stay in hand — '公開する' reveals without discarding"
    );

    // 2 energy consumed
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before - 2,
        "Q237: 2 energy should be consumed"
    );
}

/// No matching live card in waitroom → ability resolves but nothing is recovered.
///
/// Verifies:
///   - No card is moved to hand
///   - Revealed card stays in hand
///   - 2 energy consumed
#[test]
fn hinoshita_no_matching_live_in_discard_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_live = game.id("PL!HS-bp1-019-L"); // "Dream Believers"
    let wrong_live = game.id("PL!-sd1-019-SD"); // different live card, name doesn't contain "Dream Believers"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal_live);
    game.state.player1.waitroom.cards.push(wrong_live);
    game.give_energy(2);
    let energy_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(hino);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // wrong_live should NOT be in hand
    assert!(
        !game.state.player1.hand.cards.contains(&wrong_live),
        "Mismatched live card should not be recovered"
    );

    // wrong_live stays in waitroom
    assert!(
        game.state.player1.waitroom.cards.contains(&wrong_live),
        "Mismatched live card should remain in waitroom"
    );

    // Revealed card must stay in hand
    assert!(
        game.state.player1.hand.cards.contains(&reveal_live),
        "Revealed card must stay in hand — '公開する' reveals without discarding"
    );

    // 2 energy consumed
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before - 2,
        "2 energy should be consumed"
    );
}

/// Multiple matching live cards in hand → reveal cost creates a choice.
/// This verifies the choice path doesn't discard the selected card.
///
/// Scenario: 2 live cards "Dream Believers" in hand, reveal 1 (auto-select
/// won't trigger because 2 > count=1). Player picks index [0].
#[test]
fn hinoshita_reveal_choice_keeps_card_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_a = game.id("PL!HS-bp1-019-L"); // "Dream Believers"
    let reveal_b = game.new_id("PL!HS-bp1-019-L"); // another copy of "Dream Believers"
    let target_live = game.id("PL!HS-bp5-017-L"); // "Dream Believers (104th Ver.)"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal_a);
    game.state.player1.hand.cards.push(reveal_b);
    game.state.player1.waitroom.cards.push(target_live);
    game.give_energy(2);

    game.activate_ability(hino);

    // First choice: select which live card to reveal from hand (2 > count=1)
    // Choose index 0 (reveal_a)
    assert!(
        game.has_pending_choice(),
        "Should prompt to select reveal card"
    );
    game.select_indices(&[0]);

    // Second choice: select which card to recover from waitroom
    // (Only 1 match, so this auto-selects — no choice)
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Target recovered
    assert!(
        game.state.player1.hand.cards.contains(&target_live),
        "Should recover Dream Believers (104th Ver.)"
    );

    // BOTH revealed cards must still be in hand (only revealed, not discarded)
    assert!(
        game.state.player1.hand.cards.contains(&reveal_a),
        "Chosen reveal card must stay in hand"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal_b),
        "Unchosen reveal card must stay in hand"
    );
}

/// Multiple live cards in waitroom that all match the name constraint →
/// player must choose which 1 to recover.
///
/// Scenario: Reveal "Dream Believers" with "Dream Believers（104期Ver.）" AND
/// "Dream Believers（105期Ver.）" in waitroom. Both contain "Dream Believers",
/// so count=1 triggers a selection choice.
#[test]
fn hinoshita_multiple_waitroom_matches_prompts_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_live = game.id("PL!HS-bp1-019-L"); // "Dream Believers"
    let target_a = game.id("PL!HS-bp5-017-L"); // "Dream Believers（104期Ver.）"
    let target_b = game.new_id("PL!HS-sd1-018-SD"); // "Dream Believers（105期Ver.）"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal_live);
    game.state.player1.waitroom.cards.push(target_a);
    game.state.player1.waitroom.cards.push(target_b);
    game.give_energy(2);

    game.activate_ability(hino);

    // First choice: which card to recover from waitroom (2 matches > count=1)
    assert!(
        game.has_pending_choice(),
        "Should prompt to select which card to recover"
    );
    game.select_indices(&[0]); // pick first match

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Exactly 1 card should have been moved to hand
    let hand_has_a = game.state.player1.hand.cards.contains(&target_a);
    let hand_has_b = game.state.player1.hand.cards.contains(&target_b);
    assert!(
        hand_has_a || hand_has_b,
        "One of the matching cards should be recovered to hand"
    );
    assert!(
        !(hand_has_a && hand_has_b),
        "Only 1 card should be recovered — ability says '1枚'"
    );

    // Revealed card stays in hand
    assert!(
        game.state.player1.hand.cards.contains(&reveal_live),
        "Revealed card must stay in hand"
    );
}

/// Use limit: ターン1回 (once per turn). After first activation, the second
/// should be blocked until next turn.
#[test]
fn hinoshita_use_limit_once_per_turn() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal_a = game.id("PL!HS-bp1-019-L"); // "Dream Believers"
    let reveal_b = game.new_id("PL!HS-bp1-019-L"); // second copy
    let target_a = game.id("PL!HS-bp5-017-L"); // "Dream Believers（104期Ver.）"
    let target_b = game.new_id("PL!HS-bp5-017-L"); // second copy

    game.state.player1.stage.stage[1] = hino;
    // Hand: 2 live cards for 2 activations
    game.state.player1.hand.cards.push(reveal_a);
    game.state.player1.hand.cards.push(reveal_b);
    // Waitroom: 2 matching targets
    game.state.player1.waitroom.cards.push(target_a);
    game.state.player1.waitroom.cards.push(target_b);
    game.give_energy(4);

    // === First activation: should succeed ===
    game.activate_ability(hino);

    // Reveal choice: 2 cards in hand > count=1
    assert!(
        game.has_pending_choice(),
        "Should prompt to select reveal card"
    );
    game.select_indices(&[0]);

    // Recovery: 2 matches in waitroom > count=1
    assert!(
        game.has_pending_choice(),
        "Should prompt to select recovery target"
    );
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let first_hand_count = game.state.player1.hand.cards.len();
    assert!(
        first_hand_count >= 2,
        "Should have at least original reveal count + 1 recovered card in hand"
    );

    // === Second activation: should be blocked by use_limit ===
    game.give_energy(2); // refill energy
    let err = game.try_activate_ability(hino);
    assert!(
        err.is_err(),
        "Second activation in same turn should fail — use_limit=1 (ターン1回)"
    );
}

/// No live card in hand → cannot pay reveal cost → ability silently fails.
/// Cost validation happens asynchronously in the ability queue, so
/// try_activate_ability returns Ok but the cost resolver logs the failure.
#[test]
fn hinoshita_no_live_card_in_hand_fails() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hino = game.id("PL!HS-bp5-001-P");
    let target_live = game.id("PL!HS-bp5-017-L"); // "Dream Believers（104期Ver.）"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.waitroom.cards.push(target_live);
    game.give_energy(2);

    game.activate_ability(hino);

    // The ability processes asynchronously — cost fails (no card to reveal).
    // No choices should appear (cost never paid, effect never starts).
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Target should stay in waitroom (no card moved)
    assert!(
        game.state.player1.waitroom.cards.contains(&target_live),
        "Target should remain in waitroom — cost was not paid"
    );

    // No card was added to hand
    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "No cards should be in hand — no live card to reveal"
    );
}

// ============================
// Cross-card name matching: Butterfly / Butterfly Wing
// ============================

/// Reveal "Butterfly" (short name) → should recover "Butterfly Wing"
/// because "Butterfly Wing" contains "Butterfly".
#[test]
fn butterfly_to_butterfly_wing_matches() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!N-bp1-028-L"); // "Butterfly"
    let target = game.id("PL!SP-pb2-046-L"); // "Butterfly Wing"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Butterfly Wing should be recoverable — 'Butterfly Wing' contains 'Butterfly'"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Butterfly' must stay in hand"
    );
}

/// Reveal "Butterfly Wing" (longer name) → should NOT recover "Butterfly"
/// because "Butterfly" does NOT contain "Butterfly Wing".
#[test]
fn butterfly_wing_to_butterfly_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!SP-pb2-046-L"); // "Butterfly Wing"
    let target = game.id("PL!N-bp1-028-L"); // "Butterfly"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&target),
        "'Butterfly' should NOT be recoverable — 'Butterfly' does not contain 'Butterfly Wing'"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "'Butterfly' should remain in waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Butterfly Wing' must stay in hand"
    );
}

// ============================
// Cross-card name matching: Dream Believers + (105期Ver.)
// ============================

/// Q236 variant: Reveal "Dream Believers" → recover "Dream Believers（105期Ver.）"
#[test]
fn dream_believers_to_105_matches() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!HS-bp1-019-L"); // "Dream Believers"
    let target = game.id("PL!HS-sd1-018-SD"); // "Dream Believers（105期Ver.）"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "'Dream Believers（105期Ver.）' should be recoverable — contains 'Dream Believers'"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Dream Believers' must stay in hand"
    );
}

/// Q237 variant: Reveal "Dream Believers（105期Ver.）" → cannot recover "Dream Believers"
#[test]
fn dream_believers_105_to_base_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!HS-sd1-018-SD"); // "Dream Believers（105期Ver.）"
    let target = game.id("PL!HS-bp1-019-L"); // "Dream Believers"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&target),
        "'Dream Believers' should NOT be recoverable — does not contain suffix"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "'Dream Believers' should remain in waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Dream Believers（105期Ver.）' must stay in hand"
    );
}

/// Cross-variant: Reveal "Dream Believers（104期Ver.）" → cannot recover
/// "Dream Believers（105期Ver.）" because neither suffix contains the other.
#[test]
fn dream_believers_104_to_105_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!HS-bp5-017-L"); // "Dream Believers（104期Ver.）"
    let target = game.id("PL!HS-sd1-018-SD"); // "Dream Believers（105期Ver.）"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&target),
        "'Dream Believers（105期Ver.）' should NOT be recoverable from '104期Ver.' reveal"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "'Dream Believers（105期Ver.）' should remain in waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Dream Believers（104期Ver.）' must stay in hand"
    );
}

/// Cross-variant reverse: Reveal "Dream Believers（105期Ver.）" → cannot recover
/// "Dream Believers（104期Ver.）".
#[test]
fn dream_believers_105_to_104_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!HS-sd1-018-SD"); // "Dream Believers（105期Ver.）"
    let target = game.id("PL!HS-bp5-017-L"); // "Dream Believers（104期Ver.）"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&target),
        "'Dream Believers（104期Ver.）' should NOT be recoverable from '105期Ver.' reveal"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "'Dream Believers（104期Ver.）' should remain in waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Dream Believers（105期Ver.）' must stay in hand"
    );
}

// ============================
// Cross-card name matching: Link to the FUTURE
// ============================

/// Reveal "Link to the FUTURE" → recover "Link to the FUTURE（104期Ver.）"
#[test]
fn link_to_the_future_to_104_matches() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!HS-bp2-020-L"); // "Link to the FUTURE"
    let target = game.id("PL!HS-sd1-020-SD"); // "Link to the FUTURE（104期Ver.）"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&target),
        "'Link to the FUTURE（104期Ver.）' should be recoverable — contains 'Link to the FUTURE'"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Link to the FUTURE' must stay in hand"
    );
}

/// Reveal "Link to the FUTURE（104期Ver.）" → cannot recover "Link to the FUTURE"
#[test]
fn link_to_the_future_104_to_base_no_match() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hino = game.id("PL!HS-bp5-001-P");
    let reveal = game.id("PL!HS-sd1-020-SD"); // "Link to the FUTURE（104期Ver.）"
    let target = game.id("PL!HS-bp2-020-L"); // "Link to the FUTURE"

    game.state.player1.stage.stage[1] = hino;
    game.state.player1.hand.cards.push(reveal);
    game.state.player1.waitroom.cards.push(target);
    game.give_energy(2);

    game.activate_ability(hino);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !game.state.player1.hand.cards.contains(&target),
        "'Link to the FUTURE' should NOT be recoverable — does not contain suffix"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "'Link to the FUTURE' should remain in waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&reveal),
        "Revealed 'Link to the FUTURE（104期Ver.）' must stay in hand"
    );
}
