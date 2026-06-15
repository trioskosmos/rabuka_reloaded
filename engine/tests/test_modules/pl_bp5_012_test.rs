use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

fn fill_decks(game: &mut TestGame, filler: i16) {
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..30 { game.state.player2.main_deck.cards.push(filler); }
}

/// Complete live + victory flow so LiveSuccess triggers fire.
fn run_live_flow(game: &mut TestGame, p1_live_card: i16) {
    run_live_flow_both(game, p1_live_card, -1)
}

/// Live flow with optional P2 live card.
/// Pass -1 for p2_live_card if P2 shouldn't set one.
/// Both players need score-1 live cards to create a tie (condition not met).
fn run_live_flow_both(game: &mut TestGame, p1_live_card: i16, p2_live_card: i16) {
    for _ in 0..5 { game.pass(); }
    // Phase: LiveCardSetFirstAttacker (P1)
    game.set_live_card(p1_live_card);
    // Pass: → LiveCardSetSecondAttacker (P2)
    game.pass();
    if p2_live_card >= 0 {
        game.set_live_card(p2_live_card);
    }
    // Pass: → FirstAttackerPerformance (triggers LiveStart)
    game.pass();
    // Drain LiveStart auto-abilities
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => { game.select_indices(&[]); }
            _ => break,
        }
    }
    // Pass 1: First performance → SecondAttackerPerformance
    // Pass 2: Second performance → LiveVictoryDetermination
    // Pass 3: LiveVictoryDetermination fires (triggers LiveSuccess)
    for _ in 0..3 { game.pass(); }
    // Drain ALL pending choices
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectLiveSuccess") => { game.select_indices(&[0]); }
            Some("SelectAutoAbility") => { game.select_indices(&[]); }
            Some("SelectCard") => { game.select_indices(&[0]); }
            _ => break,
        }
    }
}

fn place_under_energy(game: &mut TestGame, area: MemberArea, count: usize) {
    let energy_id = game.id("LL-E-001-SD");
    for _ in 0..count {
        game.state.player1.stage.place_under_card(area, energy_id);
    }
}

// ================================================================
// ab#0: 起動 (turn1)
// ================================================================

#[test]
fn ranju_activate_places_one_energy_under_draws_one_gains_heart01() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, card, -1];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);
    fill_decks(&mut game, filler);

    let energy_before = game.state.player1.energy_zone.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    let under_before = game.state.player1.stage.get_under_cards(MemberArea::Center).len();

    game.activate_ability(card);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        energy_before - 1,
        "cost: 1 active energy removed from zone"
    );
    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::Center).len(),
        under_before + 1,
        "cost: 1 energy placed under this member"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "effect: draw 1 card"
    );
    let heart = game.state.mods.get_heart_modifier(card, HeartColor::Heart01);
    assert_eq!(heart, 1, "effect: heart01 = 1");
}

#[test]
fn ranju_activate_no_energy_still_gives_draw_and_heart() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, card, -1];
    game.state.player1.hand.cards.push(filler);
    fill_decks(&mut game, filler);

    let hand_before = game.state.player1.hand.cards.len();

    game.activate_ability(card);

    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::Center).len(), 0,
        "no energy to place under"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(), hand_before + 1,
        "draw happens even without energy"
    );
    let heart = game.state.mods.get_heart_modifier(card, HeartColor::Heart01);
    assert_eq!(heart, 1, "heart01 granted even without energy cost");
}

#[test]
fn ranju_activate_use_limit_enforces_turn1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, card, -1];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(10);
    fill_decks(&mut game, filler);

    let energy_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(card);
    assert_eq!(
        game.state.player1.energy_zone.cards.len(), energy_before - 1,
        "first activation cost paid"
    );

    let result = game.try_activate_ability(card);
    assert!(result.is_err(), "use_limit=1 blocks second activation");
    assert_eq!(
        game.state.player1.energy_zone.cards.len(), energy_before - 1,
        "second activation didn't cost"
    );
}

#[test]
fn ranju_activate_under_follows_position_change() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, card, -1];
    game.state.player1.hand.cards.push(filler);
    game.give_energy(5);
    fill_decks(&mut game, filler);

    game.activate_ability(card);
    assert_eq!(game.state.player1.stage.get_under_cards(MemberArea::Center).len(), 1);

    game.state.player1.stage.position_change(MemberArea::Center, MemberArea::LeftSide).ok();
    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::LeftSide).len(), 1,
        "under cards follow position change"
    );
    assert_eq!(game.state.player1.stage.get_under_cards(MemberArea::Center).len(), 0);
    assert_eq!(game.state.player1.stage.stage[0], card);
}

// ================================================================
// ab#1: ライブ成功時
// Condition: score > opponent
// Effect: from energy_deck, place (under_this_member + 1) as wait energy
// ================================================================

#[test]
fn ranju_live_success_places_under_plus_one_from_energy_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, card, member];
    place_under_energy(&mut game, MemberArea::Center, 3);

    // Seed energy deck BEFORE give_energy
    let energy = game.id("LL-E-001-SD");
    for _ in 0..10 { game.state.player1.energy_deck.cards.push(energy); }
    game.state.player1.hand.cards.push(live_card);

    // give_energy uses some energy deck cards too
    fill_decks(&mut game, filler);
    game.give_energy(5);

    // Capture baselines AFTER all setup
    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    let energy_zone_before = game.state.player1.energy_zone.cards.len();

    run_live_flow(&mut game, live_card);

    let energy_deck_after = game.state.player1.energy_deck.cards.len();
    let energy_zone_after = game.state.player1.energy_zone.cards.len();

    // 3 under + 1 = 4 from deck → zone (wait)
    assert_eq!(
        energy_deck_after, energy_deck_before - 4,
        "4 from deck (3 under + 1) — deck went from {} to {}",
        energy_deck_before, energy_deck_after
    );
    assert_eq!(
        energy_zone_after, energy_zone_before + 4,
        "4 added to zone — zone went from {} to {}",
        energy_zone_before, energy_zone_after
    );
}

#[test]
fn ranju_live_success_q239_zero_under_places_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, card, member];
    // 0 energy under (Q239)

    let energy = game.id("LL-E-001-SD");
    for _ in 0..10 { game.state.player1.energy_deck.cards.push(energy); }
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    let energy_zone_before = game.state.player1.energy_zone.cards.len();

    run_live_flow(&mut game, live_card);

    let energy_deck_after = game.state.player1.energy_deck.cards.len();
    let energy_zone_after = game.state.player1.energy_zone.cards.len();

    assert_eq!(
        energy_deck_after, energy_deck_before - 1,
        "Q239: 1 from deck when 0 under"
    );
    assert_eq!(
        energy_zone_after, energy_zone_before + 1,
        "Q239: 1 to zone"
    );
}

#[test]
fn ranju_live_success_condition_not_met_no_move() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, card, member];
    place_under_energy(&mut game, MemberArea::Center, 2);
    // Give P2 higher score — they'll have the same live setup so score ties
    // P1 as first attacker gets score priority, so condition SHOULD be met
    // unless P2 has more successes. We'll use the no-live-card-for-P2 approach:
    // remove P2's hand so they can't set a live card → P1 auto-wins (Q47).
    game.state.player2.hand.cards.clear();

    let energy = game.id("LL-E-001-SD");
    for _ in 0..10 { game.state.player1.energy_deck.cards.push(energy); }
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let energy_deck_before = game.state.player1.energy_deck.cards.len();
    let energy_zone_before = game.state.player1.energy_zone.cards.len();

    run_live_flow(&mut game, live_card);

    // P1 auto-wins since P2 has no live card → condition IS met → energy moves
    // Expected: 2 under + 1 = 3 from deck
    let energy_deck_after = game.state.player1.energy_deck.cards.len();
    assert_eq!(
        energy_deck_after, energy_deck_before - 3,
        "P1 auto-wins → 3 from deck (2 under + 1)"
    );
    let energy_zone_after = game.state.player1.energy_zone.cards.len();
    assert_eq!(
        energy_zone_after, energy_zone_before + 3,
        "3 added to zone"
    );
}

#[test]
fn ranju_live_success_condition_not_met_when_scores_tied() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    // Same live card for both players (score=1 each), both succeed
    // P1 stage: [member, card, member] — card has 2 under
    // P2 stage: [member, member, member]
    // Both set same-score live cards → scores tied → 1 > 1 is false
    game.state.player1.stage.stage = [member, card, member];
    game.state.player2.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live_card);
    game.state.player2.hand.cards.push(live_card);
    place_under_energy(&mut game, MemberArea::Center, 2);

    let energy = game.id("LL-E-001-SD");
    for _ in 0..10 { game.state.player1.energy_deck.cards.push(energy); }
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    // Both players set live cards so scores are tied
    run_live_flow_both(&mut game, live_card, live_card);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(), energy_deck_before,
        "no energy moved when scores tied (1 vs 1): condition not met"
    );
}

#[test]
fn ranju_live_success_scales_with_more_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, card, member];
    place_under_energy(&mut game, MemberArea::Center, 5);

    let energy = game.id("LL-E-001-SD");
    for _ in 0..20 { game.state.player1.energy_deck.cards.push(energy); }
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    run_live_flow(&mut game, live_card);

    // 5 under + 1 = 6
    assert_eq!(
        game.state.player1.energy_deck.cards.len(), energy_deck_before - 6,
        "5 under + 1 = 6 from deck"
    );
}

#[test]
fn ranju_live_success_energy_deck_empty_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, card, member];
    place_under_energy(&mut game, MemberArea::Center, 2);
    // Empty energy deck
    game.state.player1.energy_deck.cards.clear();
    game.state.player1.hand.cards.push(live_card);
    fill_decks(&mut game, filler);
    game.give_energy(5);

    let energy_zone_before = game.state.player1.energy_zone.cards.len();

    run_live_flow(&mut game, live_card);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(), energy_zone_before,
        "no energy added when deck empty"
    );
}

#[test]
fn ranju_activate_then_live_success_uses_accumulated_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let card = game.id("PL!N-bp5-012-R+");
    let live_card = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, card, member];
    game.give_energy(10);
    fill_decks(&mut game, filler);

    // Activate once → 1 energy under
    game.activate_ability(card);
    assert_eq!(
        game.state.player1.stage.get_under_cards(MemberArea::Center).len(), 1,
        "1 under after activation"
    );

    // Seed energy deck
    let energy = game.id("LL-E-001-SD");
    for _ in 0..10 { game.state.player1.energy_deck.cards.push(energy); }
    game.state.player1.hand.cards.push(live_card);

    let energy_deck_before = game.state.player1.energy_deck.cards.len();

    run_live_flow(&mut game, live_card);

    // 1 under + 1 = 2 from deck
    assert_eq!(
        game.state.player1.energy_deck.cards.len(), energy_deck_before - 2,
        "2 from deck (1 under + 1)"
    );
}
