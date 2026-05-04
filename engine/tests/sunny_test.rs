mod helpers;
use helpers::*;
use rabuka_engine::zones::MemberArea;

/// SUNNY DAY SONG (PL!-bp5-021-L) — LiveStart ability with 3 conditional branches:
///
/// Branch 1 (1+ members on stage): Both players draw 1 card, then discard 1 from hand
/// Branch 2 (2+ members on stage): 1 μ's member on your stage gains heart03 until live end
/// Branch 3 (3+ members, all distinct names): This card's score +1
///
/// Q210: Joint card (園田海未&津島善子&天王寺璃奈) counts as 1 member
/// Q211: A joint card on stage can be the target of branch 2

#[test]
fn sunny_branch1_1_member_draw_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let member = game.id("PL!-sd1-005-SD");   // 星空凛 (lilywhite)
    let filler = game.id("PL!-sd1-010-SD");   // 南ことり (Printemps)

    // Hand: sunny + filler (for discard)
    game.add_to_hand(sunny);
    game.add_to_hand(filler);
    // Stage: 1 member
    game.add_to_stage(MemberArea::Center, member);
    // Deck: cards for drawing
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);

    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();
    let p1_discard_before = game.state.player1.waitroom.cards.len();
    let p2_discard_before = game.state.player2.waitroom.cards.len();

    // Advance to live start
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 1: both draw 1, then both discard 1
    assert_eq!(game.state.player1.hand.cards.len(), p1_hand_before - 1 + 1 - 1,
        "P1 hand: -1 (played as live) +1 (draw) -1 (discard) = {} -> {}",
        p1_hand_before - 1, game.state.player1.hand.cards.len());
    assert_eq!(game.state.player2.hand.cards.len(), p2_hand_before + 1 - 1,
        "P2 hand: +1 (draw) -1 (discard)");
    assert_eq!(game.state.player1.waitroom.cards.len(), p1_discard_before + 1,
        "P1 discard: +1");
    assert_eq!(game.state.player2.waitroom.cards.len(), p2_discard_before + 1,
        "P2 discard: +1");
}

#[test]
fn sunny_branch1_no_members_does_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.add_to_hand(sunny);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);

    let p1_hand_before = game.state.player1.hand.cards.len();
    let p2_hand_before = game.state.player2.hand.cards.len();

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // No members on stage → branch 1 condition fails → no draw/discard
    assert_eq!(game.state.player1.hand.cards.len(), p1_hand_before - 1,
        "P1 hand: only -1 from playing live card");
    assert_eq!(game.state.player2.hand.cards.len(), p2_hand_before,
        "P2 hand: unchanged");
}

#[test]
fn sunny_branch3_3_members_score_plus_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD");    // 星空凛
    let kotori = game.id("PL!-sd1-010-SD");    // 南ことり
    let umi = game.id("PL!-sd1-006-SD");       // 園田海未
    let filler = game.id("PL!-sd1-013-SD");    // filler for hand

    game.add_to_hand(sunny);
    game.add_to_hand(filler);
    // Stage: 3 members with distinct names
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, kotori);
    game.add_to_stage(MemberArea::RightSide, umi);
    // Deck + opponent hand
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);
    game.state.player2.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 3: score should be +1 (3 members, distinct names)
    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 1, "Branch 3: score should be +1");

    // Verify branch 1 also fired (draw+discard)
    assert!(game.state.player2.waitroom.cards.len() >= 1,
        "P2 should have discarded a card from branch 1");
}

#[test]
fn sunny_branch3_3_members_duplicate_name_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let sunny = game.id("PL!-bp5-021-L");
    let honoka = game.id("PL!-sd1-005-SD");    // 星空凛
    let honoka2 = game.id("PL!-sd1-005-SD");   // same card (same name)
    let kotori = game.id("PL!-sd1-010-SD");    // 南ことり
    let filler = game.id("PL!-sd1-013-SD");

    game.add_to_hand(sunny);
    game.add_to_hand(filler);
    game.add_to_stage(MemberArea::Center, honoka);
    game.add_to_stage(MemberArea::LeftSide, honoka2);
    game.add_to_stage(MemberArea::RightSide, kotori);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(filler);
    game.state.player2.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(sunny);
    advance_to_live_start(&mut game);

    // Branch 3 condition: 3+ members with distinct names
    // Two members have the same name (星空凛) → distinct names fails
    let sunny_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.get_score_modifier(sunny_id);
    assert_eq!(score_mod, 0, "No score bonus when names are not all distinct");
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
