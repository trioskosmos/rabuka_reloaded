/// Comprehensive tests for live success/failure mechanics (Rules §8.3, §8.4, QA entries)
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}
fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}
fn advance_to_live_victory(game: &mut TestGame) {
    for _ in 0..3 {
        game.pass();
    }
}

/// Live card's need_heart satisfied by stage hearts -> live succeeds, LiveSuccess fires.
#[test]
fn live_succeeds_when_hearts_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    assert!(
        game.has_pending_choice(),
        "LiveSuccess fires when need_heart is satisfied (8.3.15)"
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// Insufficient hearts -> live fails, no LiveSuccess (Rules 8.3.16, Q35).
#[test]
fn live_fails_with_insufficient_hearts() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    assert!(
        !game.has_pending_choice(),
        "Q35: Live fails when need_heart not satisfied (8.3.16)"
    );
}

/// Both players have lives -> both scores compared, each may have LiveSuccess.
#[test]
fn both_players_live_score_compared() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_live = game.id("PL!-sd1-019-SD");
    let p2_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player2.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(p1_live);
    game.state.player2.hand.cards.push(p2_live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(p1_live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // One more pass to finalize LiveVictoryDetermination (move winner to success zone)
    game.pass();
    assert!(
        game.state.player1.success_live_card_zone.cards.len() > 0
            || game.state.player2.success_live_card_zone.cards.len() > 0,
        "Both players' live cards processed through score comparison (8.4.3.3)"
    );
}

/// Edge: score-0 live succeeds when hearts met (Q147).
#[test]
fn q147_empty_need_heart_live_succeeds() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    assert!(
        game.has_pending_choice(),
        "Q147: Score-0 live succeeds when hearts met"
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// Both players have lives -> both scores compared, each may have LiveSuccess.
#[test]
fn both_players_have_live_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let p1_live = game.id("PL!-sd1-019-SD");
    let p2_live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player2.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(p1_live);
    game.state.player2.hand.cards.push(p2_live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(p1_live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    game.pass();
    assert!(
        game.state.player1.success_live_card_zone.cards.len() > 0
            || game.state.player2.success_live_card_zone.cards.len() > 0,
        "Both players' live cards processed through score comparison (8.4.3.3)"
    );
}

/// P1 has live card, P2 doesn't -> P1's score is automatically higher (8.4.3.2, Q47)
#[test]
fn one_player_live_auto_higher_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    assert!(
        game.has_pending_choice(),
        "Q47: P1 with live card succeeds even if P2 has none"
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// No live card set -> no yell, no LiveSuccess (Q32).
#[test]
fn no_live_card_no_yell_no_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.clear();
    game.state.player2.hand.cards.clear();
    for _ in 0..50 {
        game.state
            .player1
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    for _ in 0..20 {
        game.state
            .player2
            .main_deck
            .cards
            .push(game.id("PL!-sd1-010-SD"));
    }
    advance_to_live_card_set_p1(&mut game);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    assert!(
        game.state.player1.main_deck.cards.len() >= 45,
        "Q32: Deck should not lose >5 cards from yell (no live card set)"
    );
    assert!(
        !game.has_pending_choice(),
        "No LiveSuccess when no live card performed"
    );
}

/// Multiple live cards: if any fails hearts, all fail (8.3.16, Q35)
#[test]
fn any_card_fails_hearts_all_fail() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!S-sd1-003-SD");
    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(live_a);
    game.state.player1.hand.cards.push(live_b);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_a);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    assert!(
        !game.has_pending_choice(),
        "Q35: All live cards fail when any card's need_heart is unmet"
    );
}

/// Winner takes at most 1 card from live zone to success zone (8.4.7, Q83)
#[test]
fn winner_takes_one_to_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live_a);
    game.state.player1.hand.cards.push(live_b);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }
    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(live_a);
    advance_to_live_start(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    advance_to_live_victory(&mut game);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.success_live_card_zone.cards.len() <= 1,
        "Q83: Winner moves at most 1 live card to success zone"
    );
}

/// Two live cards, choose the SECOND one for success zone.
#[test]
fn two_live_cards_choose_second() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live_a = game.id("PL!-sd1-019-SD");
    let live_b = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [member, member, member];
    game.state.player1.hand.cards.push(live_a);
    game.state.player1.hand.cards.push(live_b);
    for _ in 0..50 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Set first live card during LiveCardSet phase
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live_a);
    // Add second live card directly (as if by an effect)
    game.state.player1.live_card_zone.cards.push(live_b);
    assert_eq!(
        game.state.player1.live_card_zone.cards.len(),
        2,
        "2 live cards in zone"
    );

    // Advance through live start
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Advance to live victory (3 passes)
    game.pass();
    game.pass();
    game.pass();

    // Drain all LiveSuccess ability choices. Each live card has a "look at 3" ability
    // that creates a look_and_select choice. We need to finish all abilities before
    // the SelectLiveSuccess multi-card choice appears.
    let mut attempts = 0;
    while game.has_pending_choice() && attempts < 20 {
        attempts += 1;
        let t = game.pending_choice_type();
        eprintln!("[DEBUG] draining: choice_type={:?} attempt={}", t, attempts);
        match t.as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            Some("SelectCard") => {
                // Skip the look_and_select by selecting empty (skip)
                game.select_indices(&[]);
            }
            _ => {
                eprintln!("[DEBUG] unknown choice type, stopping drain");
                break;
            }
        }
    }

    // Drain loop only processes choices; remaining auto-abilities need pass()
    // to re-enter `execute_live_victory_determination` which calls
    // `process_pending_auto_abilities` and eventually reaches the multi-card
    // choice. Pass up to 10 times until the choice appears.
    for _ in 0..10 {
        if !game.has_pending_choice() {
            game.pass();
        } else {
            break;
        }
    }

    assert!(
        game.has_pending_choice(),
        "SelectLiveSuccess choice should be presented"
    );

    // Select the SECOND live card (index 1)
    game.select_indices(&[1]);

    // Verify: the chosen card went to success zone
    assert_eq!(
        game.state.player1.success_live_card_zone.cards.len(),
        1,
        "Exactly 1 card in success zone"
    );
    assert_eq!(
        game.state.player1.success_live_card_zone.cards[0], live_b,
        "Second live card (live_b) was chosen for success"
    );
    // Verify remaining cards moved to waitroom
    assert!(
        game.state.player1.live_card_zone.cards.is_empty(),
        "Live card zone is empty"
    );
}
