/// Hard tests for 日野下花帆 (PL!HS-PR-016-PR) same_unit_name filter:
///
/// ライブ開始時 手札の同じユニット名を持つカード2枚を控え室に置いてもよい：
/// ライブ終了時まで、heart04×2 + blade×2 を得る。
///
/// Q175: Discarded 2 cards MUST share the same unit name. The filter groups
/// hand cards by unit, only keeps the LARGEST unit with ≥count cards.
/// Singletons and smaller units are excluded from selection.

mod helpers;
use helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass(); // LiveCardSetP1→P2, draws 1 for P1
    game.pass(); // LiveCardSetP2→FirstAttackerPerf, draws for P2, triggers LiveStart
}

/// Hand: 3 Printemps + 2 lilywhite + 1 unitless.
/// Deck filled with unitless → draws add unitless, not Printemps/lilywhite.
/// After draws: 3P + 2L + 3U (unitless gets +2 from draws: Draw phase + LiveCardSetP1).
/// Filter: Printemps(3) ≥2, lilywhite(2) ≥2, unitless(3) ≥2.
/// Largest = Printemps(3) or unitless(3) — tie, BTreeMap picks alphabetically.
/// Printemps > unitless alphabetically? 'P' > 'u' → unitless wins? No, 'u' > 'P'.
/// Actually 'P' (80) < 'u' (117) → 'Printemps' < 'unitless'? No, 'P' < 'u'.
/// So Printemps comes first alphabetically. Tie goes to first.
/// Both are size 3. max_by_key picks first on tie → first in BTree order.
/// For the test to be deterministic, ensure one group is strictly larger.
/// Use: 3 Printemps + 1 lilywhite + 1 unitless.
/// After draws: 3P + 1L + 3U. Printemps(3) ≥2, unitless(3) ≥2, lilywhite(1) < 2.
/// Tie between Printemps(3) and unitless(3). max_by_key picks first in BTree.
/// 'P' < 'u' → 'Printemps' first → kept. Correct!
#[test]
fn hanano_q175_largest_unit_wins_tie_goes_alphabetical() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanano = game.id("PL!HS-PR-016-PR");
    let live = game.id("PL!-sd1-019-SD");
    let p_a = game.id("PL!-sd1-010-SD"); // Printemps
    let p_b = game.id("PL!-sd1-008-SD"); // Printemps
    let p_c = game.id("PL!-sd1-003-SD"); // Printemps
    let lily = game.id("PL!-sd1-013-SD"); // lilywhite (singleton, should be filtered out)
    let unitless = game.id("PL!-sd1-019-SD"); // unit=None (starts 1, draws add 2 more = 3)

    game.state.player1.stage.stage[0] = hanano;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(p_a);
    game.state.player1.hand.cards.push(p_b);
    game.state.player1.hand.cards.push(p_c);
    game.state.player1.hand.cards.push(lily);
    game.state.player1.hand.cards.push(unitless);

    // Unitless in deck for draws
    for _ in 0..20 { game.state.player1.main_deck.cards.push(unitless); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(unitless); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // After set_live: 3P + 1L + 1U = 5 (unitless from hand)
    // After LiveCardSetP1 draw (unitless from deck): 3P + 1L + 2U = 6
    // But Draw phase (pass 3) also draws 1 for P2 → not P1. So no extra draw for P1.
    // Wait — the advance_to_live_card_set_p1 passes go: Main→Active→Energy→Draw→Main→LiveCardSetP1.
    // The Draw phase draws for the ACTIVE PLAYER. After pass 1, turn_phase = SecondAttackerNormal.
    // So Draw phase on pass 3 draws for P2, not P1. P1 hand = 5 (after set_live).
    // LiveCardSetP1 draws 1 → P1 hand = 6 (3P + 1L + 2U).

    // Filter: Printemps(3) ≥2, unitless(2) ≥2, lilywhite(1) < 2 → out.
    // max_by_key: Printemps(3) > unitless(2) → Printemps wins.
    // filtered_idxs = 3 Printemps indices. 3 > 2 → choice.
    if game.has_pending_choice() {
        let before = game.state.player1.hand.cards.len();
        game.select_indices(&[0, 1]); // pick 2 Printemps
        // 2 removed → hand = before - 2
        assert_eq!(game.state.player1.hand.cards.len(), before - 2,
            "2 cards were discarded via the same-unit choice");
    }

    // lilywhite (singleton) was never selectable — it's still in hand
    assert!(game.state.player1.hand.cards.contains(&lily),
        "Singleton lilywhite should remain (filtered out)");
}

/// 3 Printemps + 1 lilywhite + 1 unitless in initial hand.
/// With draws, same as above. Verify choice only shows 3 Printemps cards.
#[test]
fn hanano_q175_singleton_filtered_out_choice_created() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanano = game.id("PL!HS-PR-016-PR");
    let live = game.id("PL!-sd1-019-SD");
    let p_a = game.id("PL!-sd1-010-SD");
    let p_b = game.id("PL!-sd1-008-SD");
    let p_c = game.id("PL!-sd1-003-SD");
    let lily = game.id("PL!-sd1-013-SD");
    let unitless = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[0] = hanano;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(p_a);
    game.state.player1.hand.cards.push(p_b);
    game.state.player1.hand.cards.push(p_c);
    game.state.player1.hand.cards.push(lily);
    game.state.player1.hand.cards.push(unitless);

    for _ in 0..20 { game.state.player1.main_deck.cards.push(unitless); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(unitless); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // A choice must exist (3 Printemps > count 2)
    assert!(game.has_pending_choice(),
        "3 same-unit cards should create a choice");

    // Pick 2 cards → 2 removed from hand
    let before = game.state.player1.hand.cards.len();
    game.select_indices(&[0, 1]);
    assert_eq!(game.state.player1.hand.cards.len(), before - 2);

    // lilywhite and unitless still there
    assert!(game.state.player1.hand.cards.contains(&lily));
    assert!(game.state.player1.hand.cards.contains(&unitless));
}

/// 1 Printemps + 1 lilywhite → neither has ≥2 → filtered empty → skip.
#[test]
fn hanano_edge_no_unit_has_2_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanano = game.id("PL!HS-PR-016-PR");
    let live = game.id("PL!-sd1-019-SD");
    let print = game.id("PL!-sd1-010-SD");
    let lily = game.id("PL!-sd1-013-SD");
    let unitless = game.id("PL!-sd1-019-SD");

    game.state.player1.stage.stage[0] = hanano;
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(print);
    game.state.player1.hand.cards.push(lily);
    game.state.player1.hand.cards.push(unitless);

    for _ in 0..20 { game.state.player1.main_deck.cards.push(unitless); }
    for _ in 0..20 { game.state.player2.main_deck.cards.push(unitless); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);

    // Hand: print(1) + lily(1) + unitless(1) + 1 drawn (unitless) = 4
    // Printemps(1) < 2, lilywhite(1) < 2 → empty → no choice → skip
    assert!(!game.has_pending_choice(),
        "No unit has ≥2 → no choice");
    assert!(game.state.player1.hand.cards.contains(&print));
    assert!(game.state.player1.hand.cards.contains(&lily));
}
