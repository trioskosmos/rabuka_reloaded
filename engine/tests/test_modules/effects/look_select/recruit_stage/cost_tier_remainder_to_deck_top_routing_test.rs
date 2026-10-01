/// Look_and_select with recruit remainder (remainder-to-deck-top shape):
/// N譫夊ｦ九ｋ 竊・1譫壹ｒ謇区惆縺ｫ蜉縺医∵ｮ九ｊ繧偵ョ繝・く縺ｮ荳翫↓謌ｻ縺吶・/// The only ability with this shape; regression home for the split
/// destination/remainder_destination parse (Issue 12).
///
/// Card: PL!HS-bp6-029-L (Proof) 窶・LiveStart: if 闢ｮ繝守ｩｺ stage cost 竕･ 20,
/// look 2 竊・take 1 to hand, rest back to deck top; if 竕･ 30, additionally
/// heart00 -2. Below 20: no effect at all.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

// ====================================================================
// Issue 12: PL!HS-bp6-029-L (Proof)
// ====================================================================
// LiveStart: if 闢ｮ繝守ｩｺ cost >= 20, look 2 竊・pick 1 to hand, rest deck_top.
// If >= 30, additionally heart00 -2.
// Parser fix: split destination (hand) + remainder_destination (deck_top).
// ====================================================================

#[test]
fn issue12_proof_look_and_select_cost_20plus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs1 = game.id("PL!HS-sd1-008-SD"); // cost 13
    let hs2 = game.id("PL!HS-bp1-004-R\u{ff0b}"); // cost 15
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs1, hs2, filler];
    game.state.player1.main_deck.cards.clear();
    let top1 = game.new_id("PL!-sd1-010-SD");
    let top2 = game.new_id("PL!-sd1-005-SD");
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    // No heart reduction (cost 28 < 30)
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "12a: cost < 30 -> no heart reduction"
    );
}

/// The 縲・0莉･荳翫・bound itself, and 窶・unlike `issue12_proof_look_and_select_cost_20plus` 窶・/// an assertion that the LOOK actually happened.
///
/// That test stages 13 + 15 = **28** and then asserts only
/// `get_need_heart_modifier(proof, Heart00) == 0`, which is a statement about the **30**
/// tier, not the 20 one. Nothing in it shows the look ran: if the 20 clause were dropped
/// entirely the assertion would still hold. Its name promises the 20 tier and the body never
/// checks it.
///
/// The bound is now staged at exactly 20 and at exactly 19, and the observable is a specific
/// card leaving the deck top for hand, which can only happen if the look fired.
///
/// Costs are pinned from `cards.json` in the assertions below rather than trusted from the
/// fixtures' comments, so a rebalanced print cannot quietly move a total across the bound.
#[test]
fn proof_cost_total_of_exactly_twenty_triggers_the_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    // 9 + 9 + 2 = exactly 20
    let a = game.id("PL!HS-sd1-016-SD");
    let b = game.id("PL!HS-sd1-001-SD");
    let c = game.id("PL!HS-sd1-009-SD");
    let filler = game.id("PL!-sd1-010-SD");

    let total: i32 = [a, b, c]
        .iter()
        .map(|&id| game.db.get_card(id).and_then(|k| k.cost).unwrap_or(0) as i32)
        .sum();
    assert_eq!(
        total, 20,
        "precondition: the staged 闢ｮ繝守ｩｺ members must total EXACTLY the printed 20莉･荳・bound \
         (got {total}); otherwise this is not the boundary case"
    );

    // A single distinctive card on the deck top: reaching hand is proof the look ran.
    let prize = game.id("PL!HS-pb1-023-N");
    game.state.player1.stage.stage = [a, b, c];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(prize);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(proof);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    // Drain ONLY the auto-ability prompts, then inspect what remains. The look's own prompt
    // is a SelectCard over the looked-at cards; anything else means the 20 clause did not fire.
    //
    // Two observables were wrong here first, recorded so they are not reused:
    // `hand.contains(prize)` is ALSO written by the LiveStart draw, so it reported a fire at
    // every total including 6; and `looked_at_cards` is emptied by answering the selection.
    // Neither can distinguish "the look ran" from "something else moved a card".
    let mut guard = 0;
    while game.pending_choice_type().as_deref() == Some("SelectAutoAbility") && guard < 16 {
        guard += 1;
        game.select_indices(&[]);
    }
    let prompt = game.pending_choice_type();

    assert_eq!(
        prompt.as_deref(),
        Some("SelectCard"),
        "コストが合計20以上 — the members total exactly 20, so the look must present its \
         1枚を手札に加え selection. No SelectCard means the gate blocked a satisfied clause \
         (prompt was {prompt:?})"
    );
    assert_eq!(
        game.pending_choice_option_count(),
        Some(2),
        "デッキの上のカードを2枚見る — the selection offers both looked-at cards"
    );
    game.drain_choices();
}

/// Threshold sweep across seven stage-cost totals, asserting where the 「合計20以上」 gate sits.
///
/// This started as an `#[ignore]`d `println!` sweep, and the reason it was ignored is now
/// gone. The old version snapshotted the prompt immediately, so the 19 and 21 rows read as
/// "queued behind a `SelectAutoAbility`" — Proof's own 自動 fires on the same phase — and a
/// single snapshot could not separate a gated look from an un-answered one. Declining the
/// stage's auto abilities first (the same thing `drain_choices` does for them) leaves the
/// look's own prompt as the next one, and then the rows separate cleanly.
///
/// It is asserted rather than printed now, which is the whole point: a `println!` diagnostic
/// cannot fail, so the sweep documented the gate while being incapable of noticing the gate
/// moving.
///
/// The result is the table the prose always claimed, and it is unchanged:
/// ```text
///   6 → no look    10 → no look    12 → no look    13 → no look    19 → no look
///  21 → look       27 → look
/// ```
/// with the 19/20 bracket additionally pinned on its own by
/// `proof_cost_total_of_nineteen_does_not_trigger_the_look` and
/// `proof_cost_total_of_exactly_twenty_…`, so any off-by-one of size 1 is caught twice.
#[test]
fn proof_threshold_table_across_seven_totals() {
    // 蓮ノ空 member costs available (from cards.json): 2, 4, 9, 11, 13, 15.
    // `expect_look` is the gate: the look opens at a combined stage cost of 20 or more.
    let cases: [(&[&str], i32, bool); 7] = [
        (&["PL!HS-sd1-009-SD", "PL!HS-sd1-009-SD", "PL!HS-sd1-009-SD"], 6, false), // 2+2+2
        (&["PL!HS-sd1-009-SD", "PL!HS-sd1-011-SD", "PL!HS-sd1-011-SD"], 10, false), // 2+4+4
        (&["PL!HS-sd1-011-SD", "PL!HS-sd1-011-SD", "PL!HS-sd1-011-SD"], 12, false), // 4+4+4
        (&["PL!HS-sd1-009-SD", "PL!HS-sd1-009-SD", "PL!HS-sd1-001-SD"], 13, false), // 2+2+9
        (&["PL!HS-sd1-009-SD", "PL!HS-sd1-011-SD", "PL!HS-sd1-008-SD"], 19, false), // 2+4+13
        (&["PL!HS-sd1-008-SD", "PL!HS-sd1-011-SD", "PL!HS-sd1-011-SD"], 21, true), // 13+4+4
        (&["PL!HS-sd1-001-SD", "PL!HS-sd1-001-SD", "PL!HS-sd1-001-SD"], 27, true), // 9+9+9
    ];

    for (triple, expect_total, expect_look) in cases {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let proof = game.id("PL!HS-bp6-029-L");
        let filler = game.id("PL!-sd1-010-SD");
        let ids: Vec<i16> = triple.iter().map(|n| game.id(n)).collect();
        let total: i32 = ids
            .iter()
            .map(|&id| game.db.get_card(id).and_then(|k| k.cost).unwrap_or(0) as i32)
            .sum();
        assert_eq!(
            total, expect_total,
            "premise: {} should total {expect_total}, got {total}",
            triple.join(" + ")
        );

        let prize = game.id("PL!HS-pb1-023-N");
        game.state.player1.stage.stage = [ids[0], ids[1], ids[2]];
        game.state.player1.main_deck.cards.clear();
        game.state.player1.main_deck.cards.push(prize);
        for _ in 0..10 {
            game.state.player1.main_deck.cards.push(filler);
            game.state.player2.main_deck.cards.push(filler);
        }
        game.state.player1.hand.cards.push(proof);

        advance_to_live_card_set_p1(&mut game);
        game.set_live_card(proof);
        advance_to_live_start(&mut game);

        // Clear the stage's own 自動 first. The previous version snapshotted the prompt
        // immediately, so the 19 and 21 rows read as "queued behind a SelectAutoAbility"
        // and the sweep could not tell a gated look from an un-answered one. Declining the
        // stage's abilities is the same thing `drain_choices` does for them, and it leaves
        // the look's own prompt as the next one.
        let mut guard = 0;
        while game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            guard += 1;
            assert!(guard <= 20, "auto-ability prompts never settled for {}", triple.join(" + "));
            game.select_indices(&[]);
        }

        // The prompt is the only sound observable — see the module header: `hand` and
        // `looked_at_cards` both lie here (the LiveStart draw reaches hand regardless, and
        // answering empties the looked-at pool).
        let prompt = game.pending_choice_type();
        let options = game.pending_choice_option_count();
        let fired = prompt.as_deref() == Some("SelectCard") && options.unwrap_or(0) > 0;

        assert_eq!(
            fired, expect_look,
            "stage cost {total} ({}) -> look fired={fired} (prompt={prompt:?} options={options:?}); \
             the gate is 合計コスト20以上, so {expect_look} is required",
            triple.join(" + ")
        );

        game.drain_choices();
    }
}

/// The sharpest negative: a total of exactly **19**.
///
/// Paired with the test above (20 → the look's SelectCard appears) this brackets the
/// comparison with a single cost of difference, so any off-by-one of size 1 is caught.
///
/// **This test was wrong twice before it was right**, and both mistakes are worth recording
/// because they were confident, not vague:
///   1. Its first version asserted `!hand.contains(prize)`. It failed — which I reported as a
///      live engine defect, an off-by-one with the threshold in 10..=19. That was **false**.
///      The LiveStart draw also writes to hand, so the prize arrived there without this look.
///   2. Its second version read `looked_at_cards.len()`, which answering the selection
///      empties. Reading the prompt BEFORE answering anything is the only sound observable,
///      and it shows the engine behaving correctly: below 20 there is no prompt at all.
///
/// The engine was right the whole time. See docs/EDGE_CASE_FINDINGS.md.
#[test]
fn proof_cost_total_of_nineteen_does_not_trigger_the_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    // 13 + 4 + 2 = exactly 19
    let a = game.id("PL!HS-sd1-008-SD");
    let b = game.id("PL!HS-sd1-012-SD");
    let c = game.id("PL!HS-sd1-015-SD");
    let filler = game.id("PL!-sd1-010-SD");

    let total: i32 = [a, b, c]
        .iter()
        .map(|&id| game.db.get_card(id).and_then(|k| k.cost).unwrap_or(0) as i32)
        .sum();
    assert_eq!(
        total, 19,
        "precondition: the staged 闢ｮ繝守ｩｺ members must total EXACTLY 19, one below the bound \
         (got {total})"
    );

    let prize = game.id("PL!HS-pb1-023-N");
    game.state.player1.stage.stage = [a, b, c];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(prize);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.hand.cards.push(proof);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    // Drain ONLY the auto-ability prompts, then assert the look never opened one. Same
    // observable discipline as the 20 case: the question is "did the SelectCard appear",
    // not "did some card move" — the LiveStart draw writes to hand too.
    let mut guard = 0;
    while game.pending_choice_type().as_deref() == Some("SelectAutoAbility") && guard < 16 {
        guard += 1;
        game.select_indices(&[]);
    }
    let prompt = game.pending_choice_type();

    assert_ne!(
        prompt.as_deref(),
        Some("SelectCard"),
        "合計19 < 20 — the look must not fire, so no card-selection prompt may appear \
         (prompt was {prompt:?})"
    );
    assert!(
        game.state.looked_at_cards.is_empty(),
        "and nothing may be left revealed — 2枚見る did not happen (looked_at={:?})",
        game.state.looked_at_cards
    );
}

#[test]
fn issue12_proof_look_and_select_cost_30plus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs_a = game.id("PL!HS-bp1-004-R\u{ff0b}"); // cost 15
    let hs_b = game.id("PL!HS-bp5-002-R\u{ff0b}"); // cost 15
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs_a, hs_b, filler];
    game.state.player1.main_deck.cards.clear();
    let top1 = game.new_id("PL!-sd1-010-SD");
    let top2 = game.new_id("PL!-sd1-005-SD");
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    let h00 = game
        .state
        .mods
        .get_need_heart_modifier(proof, HeartColor::Heart00);
    // Card says: "30莉･荳翫・蝣ｴ蜷医√＆繧峨↓蠢・ｦ√ワ繝ｼ繝医ｒheart0x2貂帙ｉ縺・
    assert_eq!(h00, -2, "12b: cost >= 30 -> heart00 -2, got {}", h00);
}

// ====================================================================
// Proof edge case: cost < 20 竊・no effect at all
// ====================================================================

#[test]
fn proof_cost_below_20_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    let hs_low = game.id("PL!HS-bp1-005-PR"); // cost=9
    let filler = game.id("PL!-sd1-010-SD");

    let filler_copy2 = game.new_id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [hs_low, filler, filler_copy2];
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }

    // Cost 9 < 20 竊・no effect at all. Hand should have only proof.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Proof: cost=9 < 20 竊・should draw nothing"
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "Proof: cost=9 < 20 竊・no heart reduction"
    );
}

// ====================================================================
// Proof edge case: cost >=20 竊・look-and-select draws 1 card
// ====================================================================

#[test]
fn proof_cost_20plus_draws_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let proof = game.id("PL!HS-bp6-029-L");
    // Use abilityless 闢ｮ繝守ｩｺ members (no LiveStart to interfere):
    //   PL!HS-bp1-016-PR cost=9, PL!HS-bp1-016-N cost=9, PL!HS-bp1-012-PR cost=4
    //   Total: 9+9+4=22 >= 20
    let hs1 = game.id("PL!HS-bp1-016-PR");
    let hs2 = game.id("PL!HS-bp1-016-N");
    let hs3 = game.id("PL!HS-bp1-012-PR");
    // Use filler for remaining cards
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [hs1, hs2, hs3];
    game.state.player1.main_deck.cards.clear();
    // Top 2 must be 闢ｮ繝守ｩｺ cards (select_action has group_names filter)
    let top1 = game.id("PL!HS-bp1-012-PR"); // abilityless 闢ｮ繝守ｩｺ cost=4
    let top2 = game.id("PL!HS-bp1-012-N"); // abilityless 闢ｮ繝守ｩｺ cost=4
    game.state.player1.main_deck.cards.push(top1);
    game.state.player1.main_deck.cards.push(top2);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(proof);
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(proof);
    advance_to_live_start(&mut game);

    // Only Proof's LiveStart fires (abilityless members have none).
    // Proof's look-and-select creates a SelectCard choice 竊・pick index 0.
    let hand_after_setup = game.state.player1.hand.cards.len();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Total cost 9+9+4=22 >= 20 竊・look-and-select fires 竊・draws 1 card
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        hand_after,
        hand_after_setup + 1,
        "Proof: cost 22 >= 20 竊・should draw 1 card (hand {}=>{} )",
        hand_after_setup,
        hand_after
    );
    assert_eq!(
        game.state
            .mods
            .get_need_heart_modifier(proof, HeartColor::Heart00),
        0,
        "Proof: cost 22 < 30 竊・no heart reduction"
    );
}
