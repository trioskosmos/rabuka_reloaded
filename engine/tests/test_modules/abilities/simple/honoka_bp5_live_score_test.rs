/// Tests for PL!-bp5-001-R＋ / P / AR / SEC (高坂穂乃果) ab#0 — LiveSuccess:
///   手札を1枚控え室に置いてもよい：自分のデッキの上から、自分のライブの
///   合計スコアに2を足した数に等しい枚数見る。その中からカードを1枚手札に加える。
///   残りを控え室に置く。
///
/// Ability: [LiveSuccess] You may discard 1 card from hand:
///   Look at (your total live score + 2) cards from top of deck.
///   Add 1 card to hand, send the rest to waitroom.
///
/// Key behavior: dynamic_count references "total_live_score" + calculation "add" 2.
///   total_live_score = sum of scores of cards in live_card_zone (NOT success zone).
///   BUG WAS: resolve_dynamic_count read from success_live_card_zone which is empty
///   during a live — should read from live_card_zone.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn trigger_live_success(game: &mut TestGame, card_id: i16) {
    fire_trigger(
        game,
        card_id,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
}

/// Setup: Honoka on stage, hand cards, deck filled with known filler cards.
/// Returns (honoka_id, deck_before_count).
fn setup_honoka(
    game: &mut TestGame,
    honoka_card_no: &str,
    hand_count: usize,
    deck_count: usize,
) -> (i16, usize) {
    let honoka = game.id(honoka_card_no);
    let filler = game.id_ref("PL!-sd1-010-SD");

    game.state.player1.stage.stage[0] = honoka;

    for _ in 0..hand_count {
        game.state
            .player1
            .hand
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }

    game.state.player1.main_deck.cards.clear();
    for _ in 0..deck_count {
        game.state.player1.main_deck.cards.push(filler);
    }

    let deck_before = game.state.player1.main_deck.cards.len();
    (honoka, deck_before)
}

/// Pay the optional cost. OBSERVED on every variant: the cost is offered
/// directly as a skippable SelectCard ("Select 1 card(s) from hand
/// (or skip)") — no SelectTarget gate precedes it.
fn pay_optional_cost(game: &mut TestGame) {
    assert!(game.has_pending_choice(), "Should have cost choice");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard skippable discard-cost prompt"
    );
    game.select_indices(&[0]);
}

/// Pay cost and return the looked_at_cards count.
fn pay_cost_and_get_look_count(game: &mut TestGame) -> usize {
    pay_optional_cost(game);
    assert!(
        game.has_pending_choice(),
        "Should have look_and_select choice after paying cost"
    );
    game.state.looked_at_cards.len()
}

/// Answer the outstanding looked_at pick with [0], then drain any trailing
/// SelectAutoAbility prompts.
fn drain_after_look(game: &mut TestGame) {
    assert!(
        game.has_pending_choice(),
        "looked_at pick must still be pending"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard looked_at prompt"
    );
    game.select_indices(&[0]);
    while game.has_pending_choice() {
        let ct = game.pending_choice_type();
        match ct.as_deref() {
            Some("SelectAutoAbility") => {
                game.select_indices(&[]);
            }
            _ => break,
        }
    }
}

/// Skip the optional cost entirely. OBSERVED: skippable SelectCard; the
/// empty answer declines.
fn skip_optional_cost(game: &mut TestGame) {
    assert!(game.has_pending_choice(), "Should have cost choice");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard skippable discard-cost prompt"
    );
    game.select_indices(&[]);
}

// ---------------------------------------------------------------------------
// Test: score=1 → look at 3 (1+2)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_score_1_looks_at_3() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    let live = game.id("PL!-sd1-019-SD"); // score=1
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "score=1 → 1+2 = 3 cards looked at");

    drain_after_look(&mut game);

    // hand: 3 start - 1 cost + 1 picked = 3
    assert_eq!(game.state.player1.hand.cards.len(), 3);
    // waitroom: 2 remaining looked_at + 1 cost = 3
    assert_eq!(game.state.player1.waitroom.cards.len(), 3);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 3);
    assert!(game.state.player1.stage.stage.contains(&honoka));
}

// ---------------------------------------------------------------------------
// Test: score=2 → look at 4 (2+2)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_score_2_looks_at_4() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    let live2 = game.id("PL!-sd1-020-SD"); // score=2
    game.state.player1.live_card_zone.cards.push(live2);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 4, "score=2 → 2+2 = 4 cards looked at");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 4);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 4);
}

// ---------------------------------------------------------------------------
// Test: score=1+2=3 → look at 5 (3+2)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_score_3_looks_at_5() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    let live1 = game.id("PL!-sd1-019-SD"); // score=1
    let live2 = game.id("PL!-sd1-020-SD"); // score=2
    game.state.player1.live_card_zone.cards.push(live1);
    game.state.player1.live_card_zone.cards.push(live2);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 5, "score=1+2=3 → 3+2 = 5 cards looked at");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 5);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 5);
}

// ---------------------------------------------------------------------------
// Test: score=0 (empty live zone) → look at 2 (0+2)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_score_0_looks_at_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 2, "score=0 → 0+2 = 2 cards looked at");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 2);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 2);
}

// ---------------------------------------------------------------------------
// Test: score=4 (4x score=1 cards) → look at 6 (4+2)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_score_4_looks_at_6() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    for _ in 0..4 {
        let live = game.id("PL!-sd1-019-SD"); // score=1
        game.state.player1.live_card_zone.cards.push(live);
    }

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 6, "score=4 → 4+2 = 6 cards looked at");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 6);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 6);
}

// ---------------------------------------------------------------------------
// Test: skip optional cost → effect does not fire
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_skip_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    skip_optional_cost(&mut game);

    assert!(
        !game.has_pending_choice(),
        "No more choices after skipping cost"
    );
    assert_eq!(game.state.player1.main_deck.cards.len(), 20);
    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert!(game.state.looked_at_cards.is_empty());
}

// ---------------------------------------------------------------------------
// Test: deck has fewer cards than score+2 → refresh mid-look
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_deck_refresh_during_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 1);

    let live = game.id("PL!-sd1-019-SD"); // score=1 → look at 3
    game.state.player1.live_card_zone.cards.push(live);

    for _ in 0..5 {
        game.state
            .player1
            .waitroom
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "Even with refresh, should look at 3 cards");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert!(game.state.player1.waitroom.cards.len() >= 2);
}

// ---------------------------------------------------------------------------
// Test: each rarity variant works (R+, P, AR, SEC)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_rplus_variant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "R+ variant: score=1 → look at 3");
}

#[test]
fn honoka_bp5_p_variant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-P", 3, 20);

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "P variant: score=1 → look at 3");
}

#[test]
fn honoka_bp5_ar_variant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-AR", 3, 20);

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "AR variant: score=1 → look at 3");
}

#[test]
fn honoka_bp5_sec_variant() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-SEC", 3, 20);

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "SEC variant: score=1 → look at 3");
}

// ---------------------------------------------------------------------------
// Test: picked card actually goes to hand
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_selected_card_goes_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 5);

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    let target = game.id("PL!-sd1-020-SD");
    game.state.player1.main_deck.cards[0] = target;

    trigger_live_success(&mut game, honoka);
    pay_optional_cost(&mut game);

    assert!(game.has_pending_choice());
    drain_after_look(&mut game);

    assert!(game.state.player1.hand.cards.contains(&target));
}

// ---------------------------------------------------------------------------
// Test: non-selected looked-at cards go to waitroom (discard_remaining)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_nonselected_goes_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 5);

    let live = game.id("PL!-sd1-019-SD"); // score=1 → look at 3
    game.state.player1.live_card_zone.cards.push(live);

    let card_a = game.id("PL!-sd1-014-SD");
    let card_b = game.id("PL!-sd1-015-SD");
    let card_c = game.id("PL!-sd1-016-SD");
    game.state.player1.main_deck.cards[0] = card_a;
    game.state.player1.main_deck.cards[1] = card_b;
    game.state.player1.main_deck.cards[2] = card_c;

    trigger_live_success(&mut game, honoka);
    pay_optional_cost(&mut game);

    assert!(game.has_pending_choice());
    assert_eq!(game.state.looked_at_cards.len(), 3);
    drain_after_look(&mut game);

    assert!(game.state.player1.hand.cards.contains(&card_a));
    assert!(game.state.player1.waitroom.cards.contains(&card_b));
    assert!(game.state.player1.waitroom.cards.contains(&card_c));
}

// ---------------------------------------------------------------------------
// Test: no cards in hand → cannot pay cost → ability fizzles
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_no_hand_cannot_pay_cost() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let honoka = game.id("PL!-bp5-001-R\u{ff0b}");

    game.state.player1.stage.stage[0] = honoka;
    game.state.player1.main_deck.cards.clear();
    for _ in 0..20 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id_ref("PL!-sd1-010-SD"));
    }

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);

    // No hand cards exist -> the optional discard cost is unpayable and
    // auto-skips (observed: no prompt at all, nothing looked at).
    assert!(
        !game.has_pending_choice(),
        "unpayable optional cost (empty hand) must auto-skip without prompting"
    );

    assert!(game.state.looked_at_cards.is_empty());
}

// ---------------------------------------------------------------------------
// Test: ability has no use_limit — can fire multiple times
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_no_use_limit_multiple_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 6, 40);

    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    let looked1 = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked1, 3, "First fire: look at 3");
    drain_after_look(&mut game);

    let hand_after_1 = game.state.player1.hand.cards.len();
    let deck_after_1 = game.state.player1.main_deck.cards.len();

    trigger_live_success(&mut game, honoka);
    let looked2 = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked2, 3, "Second fire: still look at 3");
    drain_after_look(&mut game);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_after_1 - 1 + 1,
        "Second fire: -1 cost +1 picked"
    );
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_after_1 - 3);
}

// ---------------------------------------------------------------------------
// Test: high score (score=6) → look at 8 (6+2)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_high_score_looks_at_8() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    for _ in 0..6 {
        let live = game.id("PL!-sd1-019-SD");
        game.state.player1.live_card_zone.cards.push(live);
    }

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 8, "score=6 → 6+2 = 8 cards looked at");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 8);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 8);
}

// ---------------------------------------------------------------------------
// Test: score from mixed live cards (1+2+1+2 = 6) → look at 8
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_mixed_scores_looks_correctly() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    let live1 = game.id("PL!-sd1-019-SD"); // score=1
    let live2 = game.id("PL!-sd1-020-SD"); // score=2
    game.state.player1.live_card_zone.cards.push(live1);
    game.state.player1.live_card_zone.cards.push(live2);
    game.state.player1.live_card_zone.cards.push(live1);
    game.state.player1.live_card_zone.cards.push(live2);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 8, "score=1+2+1+2=6 → 6+2 = 8 cards looked at");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 8);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 8);
}

// ---------------------------------------------------------------------------
// Test: deck has exactly score+2 cards → no refresh needed
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_deck_exactly_enough() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 3);

    let live = game.id("PL!-sd1-019-SD"); // score=1 → look at 3
    game.state.player1.live_card_zone.cards.push(live);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "Deck has exactly 3 → look at all 3");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.main_deck.cards.len(), 0);
}

// ---------------------------------------------------------------------------
// Test: deck has 1 card, score=1 → refresh then look at 3
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_deck_1_card_score_1_refreshes() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, _) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 1);

    let live = game.id("PL!-sd1-019-SD"); // score=1 → look at 3
    game.state.player1.live_card_zone.cards.push(live);

    for _ in 0..5 {
        game.state
            .player1
            .waitroom
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 3, "Refresh should provide enough cards for 3");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
}

// ---------------------------------------------------------------------------
// Test: multiple live cards in live card zone (score=1+2=3) → look at 5
// This is the REAL scenario: cards are in live_card_zone during the live,
// not in success_live_card_zone. The bug was that resolve_dynamic_count
// read from the wrong zone.
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_multiple_live_cards_in_live_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    // Simulate a real live: 3 live cards in the live_card_zone
    let live_a = game.id("PL!-sd1-019-SD"); // score=1
    let live_b = game.id("PL!-sd1-020-SD"); // score=2
    let live_c = game.id("PL!-sd1-019-SD"); // score=1
    game.state.player1.live_card_zone.cards.push(live_a);
    game.state.player1.live_card_zone.cards.push(live_b);
    game.state.player1.live_card_zone.cards.push(live_c);

    // success_live_card_zone should be EMPTY — this is the key difference
    // from the old buggy behavior
    assert!(
        game.state.player1.success_live_card_zone.cards.is_empty(),
        "success_live_card_zone must be empty during live"
    );

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    // score = 1+2+1 = 4 → 4+2 = 6
    assert_eq!(
        looked, 6,
        "score=1+2+1=4 → 4+2 = 6 cards looked at (from live_card_zone)"
    );

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 6);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 6);
}

// ---------------------------------------------------------------------------
// Test: success_live_card_zone has cards but live_card_zone is also populated
// (real game: both can have cards from previous/ongoing lives)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_reads_live_zone_not_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    // Previous lives in success zone (score=5) — should NOT affect the count
    let prev_live = game.id("PL!-sd1-020-SD"); // score=2
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(prev_live);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(prev_live);
    game.state
        .player1
        .success_live_card_zone
        .cards
        .push(prev_live); // total=6 in success zone

    // Current live (score=1) — THIS is what total_live_score should read
    let current_live = game.id("PL!-sd1-019-SD"); // score=1
    game.state.player1.live_card_zone.cards.push(current_live);

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    // score from live_card_zone = 1 → 1+2 = 3 (NOT 6+2=8 from success zone)
    assert_eq!(
        looked, 3,
        "total_live_score reads live_card_zone only, not success zone"
    );

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 3);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 3);
}

// ---------------------------------------------------------------------------
// Test: score=5 from 5 separate live cards → look at 7 (5+2)
// ---------------------------------------------------------------------------

#[test]
fn honoka_bp5_five_live_cards_looks_at_7() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (honoka, deck_before) = setup_honoka(&mut game, "PL!-bp5-001-R\u{ff0b}", 3, 20);

    for _ in 0..5 {
        let live = game.id("PL!-sd1-019-SD"); // score=1
        game.state.player1.live_card_zone.cards.push(live);
    }

    trigger_live_success(&mut game, honoka);
    let looked = pay_cost_and_get_look_count(&mut game);
    assert_eq!(looked, 7, "score=5 → 5+2 = 7 cards looked at");

    drain_after_look(&mut game);

    assert_eq!(game.state.player1.hand.cards.len(), 3);
    assert_eq!(game.state.player1.waitroom.cards.len(), 7);
    assert_eq!(game.state.player1.main_deck.cards.len(), deck_before - 7);
}
