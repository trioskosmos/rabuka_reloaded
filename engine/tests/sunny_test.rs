mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

/// SUNNY DAY SONG (PL!-bp5-021-L) — LiveStart ability with 3 conditional branches.

#[test]
fn sunny_branch1_1_member_triggers_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let member = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, member);
    // Add enough cards for phase draws + ability draw
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..5 { game.state.player2.main_deck.cards.push(filler); }
    game.state.player2.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1 requires choosing which card to discard from hand
    if game.has_pending_choice() { game.select_indices(&[0]); }

    // Branch 1 fired: at least one card was drawn (from either player's deck)
    // Verify that cards moved: opponent has hand+discard > initial
    let p2_total = game.state.player2.hand.cards.len() + game.state.player2.waitroom.cards.len();
    assert!(p2_total > 0, "P2 should have drawn + discarded cards");
    // Opponent's hand or discard changed (they drew then discarded)
    let p2_total = game.state.player2.hand.cards.len() + game.state.player2.waitroom.cards.len();
    assert!(p2_total >= 2, "P2 should have drawn + discarded, total cards >= 2");
}

#[test]
fn sunny_branch1_no_members_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(sunny);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..5 { game.state.player2.main_deck.cards.push(filler); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // No members → all conditions fail → ability does nothing.
    // Verify that none of the ability's effects triggered.
    // P1 started with 1 card (sunny), after set_live_card it may be gone.
    // If no draw happened, hand should be ≤ 1.
    assert!(game.state.player1.hand.cards.len() <= 1,
        "P1 hand should not have increased (no draw triggered)");
}

#[test]
fn sunny_branch3_3_members_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD");   // 星空凛
    let kotori = game.id("PL!-sd1-010-SD");    // 南ことり
    let umi = game.id("PL!-sd1-006-SD");       // 園田海未
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, kotori);
    game.add_to_stage(MemberArea::RightSide, umi);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..5 { game.state.player2.main_deck.cards.push(filler); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Handle any pending choice from branch 1's discard
    if game.has_pending_choice() { game.select_indices(&[0]); }

    // Branch 3: score +1 for 3 distinct-name members
    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 1, "3 distinct-name members should give +1 score");
}

#[test]
fn sunny_branch3_3_members_duplicate_name_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD");    // 星空凛
    let honoka2 = game.id("PL!-sd1-005-SD");   // same name
    let kotori = game.id("PL!-sd1-010-SD");    // 南ことり
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, honoka2);
    game.add_to_stage(MemberArea::RightSide, kotori);
    for _ in 0..5 { game.state.player1.main_deck.cards.push(filler); }
    for _ in 0..5 { game.state.player2.main_deck.cards.push(filler); }

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 0, "No score bonus with duplicate names");
}

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Active");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Energy");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Draw");
    game.pass(); assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass(); assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
