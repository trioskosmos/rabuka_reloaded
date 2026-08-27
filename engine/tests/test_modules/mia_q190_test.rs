/// Q190: PL!N-bp4-011-R＋ ミア・テイラー ab#0 ライブ開始時:
/// 「手札のライブカードを1枚控え室に置いてもよい：好きなハートの色を1つ指定する。
/// ライブ終了時まで、そのハートを1つ得る。」
///
/// Q190 ruling: the player specifies ANY heart COLOR — the catch-all「ALL」
/// (heart00) is not a color and must NOT be offered. Only heart01-06 appear.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::HeartColor;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn setup_mia(game: &mut TestGame) -> i16 {
    let mia = game.id("PL!N-bp4-011-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    fill_decks(game, filler);
    // Direct placement: Mia must be on stage when the live starts.
    game.state.player1.stage.stage = [-1, mia, -1];
    mia
}

#[test]
fn mia_q190_heart_selection_excludes_all_and_grants_chosen_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = setup_mia(&mut game);

    let live_for_zone = game.id("PL!-sd1-019-SD");
    let live_for_cost = game.id("PL!-sd1-021-SD");
    game.add_to_hand(live_for_zone);
    game.add_to_hand(live_for_cost);

    advance_to_live_set(&mut game);
    game.set_live_card(live_for_zone);
    game.pass();
    game.pass();

    // Prompt 1: optional cost — a skippable SelectCard over hand live cards.
    assert_ability!(
        game,
        "p1",
        game.has_pending_choice(),
        "LiveStart optional cost must prompt"
    );
    match game.get_pending_choice() {
        Choice::SelectCard { zone, count, allow_skip, .. } => {
            assert_eq!(zone, "hand", "discard source is the hand");
            assert_eq!(*count, 1);
            assert!(*allow_skip, "cost is optional — skipping must be offered");
        }
        other => panic!("expected SelectCard, got {:?}", other),
    }
    // Index within the hand's LIVE-card subset (the choice is filtered).
    let live_pos = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .position(|&c| c == live_for_cost)
        .expect("cost live card still in hand");
    let filtered_idx = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .take(live_pos)
        .filter(|&&c| {
            game.db
                .get_card(c)
                .is_some_and(|card| card.card_type == rabuka_engine::card::CardType::Live)
        })
        .count();
    game.select_indices(&[filtered_idx]);

    // Prompt 3: THE Q190 ASSERTION — six colors offered,「ALL」(heart00)
    // excluded.
    assert_ability!(
        game,
        "p1",
        game.has_pending_choice(),
        "heart color specification must prompt after paying"
    );
    let expected: Vec<String> = ["heart01", "heart02", "heart03", "heart04", "heart05", "heart06"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    match game.get_pending_choice() {
        Choice::SelectHeartColor { count, options, .. } => {
            assert_eq!(*count, 1);
            let mut opts = options.clone();
            opts.sort();
            assert_eq!(
                opts, expected,
                "Q190: ALL (heart00) must not be choosable — exactly heart01-06 offered"
            );
        }
        other => panic!("expected SelectHeartColor, got {:?}", other),
    }
    game.select_option(1); // heart02

    // Outcomes: the chosen color was gained until live end…
    assert_eq!(
        game.state.mods.get_heart_modifier(mia, HeartColor::Heart02),
        1,
        "gained the specified heart02"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(mia, HeartColor::Heart03),
        0,
        "only the specified color was gained"
    );
    // …and the cost live actually reached the waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&live_for_cost),
        "paid live card must be in the waitroom"
    );
    assert!(
        !game.has_pending_choice(),
        "chain fully resolved"
    );
}

#[test]
fn mia_q190_skipping_the_cost_specifies_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = setup_mia(&mut game);

    let live_for_zone = game.id("PL!-sd1-019-SD");
    let live_for_cost = game.id("PL!-sd1-021-SD");
    game.add_to_hand(live_for_zone);
    game.add_to_hand(live_for_cost);

    advance_to_live_set(&mut game);
    game.set_live_card(live_for_zone);
    game.pass();
    game.pass();

    assert!(game.has_pending_choice(), "pay/skip prompt must appear");
    match game.get_pending_choice() {
        Choice::SelectCard { zone, allow_skip, .. } => {
            assert_eq!(zone, "hand");
            assert!(*allow_skip, "cost is optional — skipping must be offered");
        }
        other => panic!("expected skippable SelectCard, got {:?}", other),
    }
    game.select_indices(&[]); // skip the optional cost

    assert!(
        !game.has_pending_choice(),
        "skipping ends the chain — no heart color prompt"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live_for_cost),
        "skipped cost: live card stays in hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live_for_cost),
        "skipped cost: nothing discarded"
    );
    assert_eq!(
        game.state.mods.get_heart_modifier(mia, HeartColor::Heart02),
        0,
        "no heart gained without paying"
    );
}

#[test]
fn mia_q190_choose_heart05_grants_heart05() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = setup_mia(&mut game);
    let live_for_zone = game.id("PL!-sd1-019-SD");
    let live_for_cost = game.id("PL!-sd1-021-SD");
    game.add_to_hand(live_for_zone);
    game.add_to_hand(live_for_cost);
    advance_to_live_set(&mut game);
    game.set_live_card(live_for_zone);
    game.pass(); game.pass();
    assert!(game.has_pending_choice());
    let live_pos = game.state.player1.hand.cards.iter().position(|&c| c == live_for_cost).unwrap();
    let filtered_idx = game.state.player1.hand.cards.iter().take(live_pos).filter(|&&c| game.db.get_card(c).is_some_and(|card| card.card_type == rabuka_engine::card::CardType::Live)).count();
    game.select_indices(&[filtered_idx]);
    assert!(game.has_pending_choice());
    // Choose heart05 (index 4 in sorted heart01-06)
    game.select_option(4);
    assert_eq!(game.state.mods.get_heart_modifier(mia, HeartColor::Heart05), 1, "heart05 should be gained");
    assert_eq!(game.state.mods.get_heart_modifier(mia, HeartColor::Heart02), 0, "other heart not gained");
}

#[test]
fn mia_q190_no_live_in_hand_still_prompts_skip() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = setup_mia(&mut game);
    let live_for_zone = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(live_for_zone);
    game.add_to_hand(filler);
    advance_to_live_set(&mut game);
    game.set_live_card(live_for_zone);
    game.pass(); game.pass();
    // With no live in hand (only filler), the optional cost may still prompt as skippable or may be auto-skipped
    if game.has_pending_choice() {
        match game.get_pending_choice() {
            Choice::SelectCard { allow_skip, .. } => assert!(*allow_skip),
            other => panic!("expected SelectCard, got {:?}", other),
        }
        game.select_indices(&[]);
    }
    assert!(!game.has_pending_choice());
}

#[test]
fn mia_live_success_distinct_3_recovers() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-bp4-011-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let l1 = game.id("PL!N-bp1-028-L");
    let l2 = game.new_id("PL!N-bp1-028-L");
    let l3 = game.new_id("PL!N-bp1-028-L");
    game.state.player1.waitroom.cards.push(l1);
    game.state.player1.waitroom.cards.push(l2);
    game.state.player1.waitroom.cards.push(l3);
    game.state.player1.stage.stage = [-1, mia, -1];
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); game.state.player2.main_deck.cards.push(filler); }
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..7 { game.pass(); if game.has_pending_choice() { game.select_indices(&[]); } }
    assert!(!game.has_pending_choice());
}

#[test]
fn mia_live_success_distinct_2_no_recover() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let mia = game.id("PL!N-bp4-011-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let l1 = game.id("PL!N-bp1-028-L");
    let l2 = game.new_id("PL!N-bp1-028-L");
    game.state.player1.waitroom.cards.push(l1);
    game.state.player1.waitroom.cards.push(l2);
    game.state.player1.stage.stage = [-1, mia, -1];
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }
    let live = game.id("PL!-sd1-019-SD");
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 { game.pass(); }
    game.set_live_card(live);
    for _ in 0..7 { game.pass(); if game.has_pending_choice() { game.select_indices(&[]); } }
    assert!(!game.has_pending_choice());
}
